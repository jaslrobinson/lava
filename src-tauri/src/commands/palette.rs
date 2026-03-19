use std::collections::HashMap;

use crate::providers::SharedProviderData;
use tauri::State;

/// Extract a color palette from an image and store it in provider data under "bp" prefix.
///
/// The formula engine resolves `$bp(path, type)$` by looking up providers["bp"]["path_type"],
/// so we write results with keys like "path/to/image_dominant", "path/to/image_vibrant", etc.
#[tauri::command]
pub async fn extract_palette(
    image_path: String,
    provider_data: State<'_, SharedProviderData>,
) -> Result<HashMap<String, String>, String> {
    let path = image_path.clone();
    let palette = tokio::task::spawn_blocking(move || compute_palette(&path))
        .await
        .map_err(|e| format!("Task error: {}", e))?
        .map_err(|e| format!("Palette extraction failed: {}", e))?;

    // Store in provider data under "bp" prefix with keys: "{image_path}_{type}"
    let mut result = HashMap::new();
    {
        let mut data = provider_data.write().await;
        let bp_map = data.entry("bp".to_string()).or_default();
        for (palette_type, hex_color) in &palette {
            let key = format!("{}_{}", image_path, palette_type);
            bp_map.insert(key.clone(), hex_color.clone());
            result.insert(palette_type.clone(), hex_color.clone());
        }
    }

    Ok(result)
}

/// Compute palette colors from an image file using median-cut quantization.
fn compute_palette(image_path: &str) -> Result<HashMap<String, String>, String> {
    use image::GenericImageView;

    let img = image::open(image_path).map_err(|e| format!("Failed to open image: {}", e))?;

    // Resize to max 100x100 for speed
    let thumb = img.resize(100, 100, image::imageops::FilterType::Triangle);
    let (width, height) = thumb.dimensions();

    // Collect all non-transparent pixels as (r, g, b)
    let mut pixels: Vec<[u8; 3]> = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let px = thumb.get_pixel(x, y);
            // Skip mostly-transparent pixels
            if px[3] < 128 {
                continue;
            }
            pixels.push([px[0], px[1], px[2]]);
        }
    }

    if pixels.is_empty() {
        // Fully transparent image -- return defaults
        let mut palette = HashMap::new();
        let default = "#808080".to_string();
        for key in &[
            "dominant",
            "vibrant",
            "muted",
            "light_vibrant",
            "dark_vibrant",
            "light_muted",
            "dark_muted",
        ] {
            palette.insert(key.to_string(), default.clone());
        }
        return Ok(palette);
    }

    // Median-cut quantization: split pixels into 8 buckets
    let quantized = median_cut(pixels, 3); // 2^3 = 8 buckets

    // Categorize the quantized colors
    let palette = categorize_colors(&quantized);

    Ok(palette)
}

/// Simple median-cut color quantization.
/// `depth` controls number of splits: produces up to 2^depth color buckets.
fn median_cut(mut pixels: Vec<[u8; 3]>, depth: usize) -> Vec<([u8; 3], usize)> {
    if depth == 0 || pixels.is_empty() {
        let count = pixels.len();
        if pixels.is_empty() {
            return vec![];
        }
        let avg = average_color(&pixels);
        return vec![(avg, count)];
    }

    // Find the channel with the widest range
    let (mut min_r, mut max_r) = (255u8, 0u8);
    let (mut min_g, mut max_g) = (255u8, 0u8);
    let (mut min_b, mut max_b) = (255u8, 0u8);

    for px in &pixels {
        min_r = min_r.min(px[0]);
        max_r = max_r.max(px[0]);
        min_g = min_g.min(px[1]);
        max_g = max_g.max(px[1]);
        min_b = min_b.min(px[2]);
        max_b = max_b.max(px[2]);
    }

    let range_r = max_r as i32 - min_r as i32;
    let range_g = max_g as i32 - min_g as i32;
    let range_b = max_b as i32 - min_b as i32;

    let sort_channel = if range_r >= range_g && range_r >= range_b {
        0
    } else if range_g >= range_b {
        1
    } else {
        2
    };

    pixels.sort_unstable_by_key(|px| px[sort_channel]);

    let mid = pixels.len() / 2;
    let right = pixels.split_off(mid);
    let left = pixels;

    let mut result = median_cut(left, depth - 1);
    result.extend(median_cut(right, depth - 1));
    result
}

/// Average color of a pixel set.
fn average_color(pixels: &[[u8; 3]]) -> [u8; 3] {
    let (mut sum_r, mut sum_g, mut sum_b) = (0u64, 0u64, 0u64);
    for px in pixels {
        sum_r += px[0] as u64;
        sum_g += px[1] as u64;
        sum_b += px[2] as u64;
    }
    let n = pixels.len() as u64;
    [
        (sum_r / n) as u8,
        (sum_g / n) as u8,
        (sum_b / n) as u8,
    ]
}

/// Compute HSL saturation and lightness for an RGB color.
fn rgb_to_sl(r: u8, g: u8, b: u8) -> (f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f64::EPSILON {
        return (0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    (s, l)
}

/// Format an RGB color as a hex string.
fn format_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

/// Categorize quantized colors into palette types:
/// dominant, vibrant, muted, light_vibrant, dark_vibrant, light_muted, dark_muted
fn categorize_colors(colors: &[([u8; 3], usize)]) -> HashMap<String, String> {
    let mut palette = HashMap::new();

    if colors.is_empty() {
        let default = "#808080".to_string();
        for key in &[
            "dominant",
            "vibrant",
            "muted",
            "light_vibrant",
            "dark_vibrant",
            "light_muted",
            "dark_muted",
        ] {
            palette.insert(key.to_string(), default.clone());
        }
        return palette;
    }

    // Annotate each color with saturation and lightness
    struct ColorInfo {
        rgb: [u8; 3],
        count: usize,
        saturation: f64,
        lightness: f64,
    }

    let mut infos: Vec<ColorInfo> = colors
        .iter()
        .map(|(rgb, count)| {
            let (s, l) = rgb_to_sl(rgb[0], rgb[1], rgb[2]);
            ColorInfo {
                rgb: *rgb,
                count: *count,
                saturation: s,
                lightness: l,
            }
        })
        .collect();

    // Dominant: most frequent color
    infos.sort_by(|a, b| b.count.cmp(&a.count));
    let dominant = &infos[0];
    palette.insert(
        "dominant".to_string(),
        format_hex(dominant.rgb[0], dominant.rgb[1], dominant.rgb[2]),
    );

    // Vibrant: highest saturation, moderate lightness (0.15 .. 0.85)
    let vibrant = infos
        .iter()
        .filter(|c| c.lightness > 0.15 && c.lightness < 0.85)
        .max_by(|a, b| a.saturation.partial_cmp(&b.saturation).unwrap())
        .unwrap_or(&infos[0]);
    palette.insert(
        "vibrant".to_string(),
        format_hex(vibrant.rgb[0], vibrant.rgb[1], vibrant.rgb[2]),
    );

    // Muted: lowest saturation
    let muted = infos
        .iter()
        .min_by(|a, b| a.saturation.partial_cmp(&b.saturation).unwrap())
        .unwrap();
    palette.insert(
        "muted".to_string(),
        format_hex(muted.rgb[0], muted.rgb[1], muted.rgb[2]),
    );

    // Light vibrant: saturated (>0.3) and light (>0.5)
    let light_vibrant = infos
        .iter()
        .filter(|c| c.saturation > 0.3 && c.lightness > 0.5)
        .max_by(|a, b| {
            let score_a = a.saturation + a.lightness;
            let score_b = b.saturation + b.lightness;
            score_a.partial_cmp(&score_b).unwrap()
        })
        .unwrap_or(vibrant);
    palette.insert(
        "light_vibrant".to_string(),
        format_hex(light_vibrant.rgb[0], light_vibrant.rgb[1], light_vibrant.rgb[2]),
    );

    // Dark vibrant: saturated (>0.3) and dark (<0.5)
    let dark_vibrant = infos
        .iter()
        .filter(|c| c.saturation > 0.3 && c.lightness < 0.5)
        .max_by(|a, b| {
            let score_a = a.saturation + (1.0 - a.lightness);
            let score_b = b.saturation + (1.0 - b.lightness);
            score_a.partial_cmp(&score_b).unwrap()
        })
        .unwrap_or(vibrant);
    palette.insert(
        "dark_vibrant".to_string(),
        format_hex(dark_vibrant.rgb[0], dark_vibrant.rgb[1], dark_vibrant.rgb[2]),
    );

    // Light muted: desaturated (<0.4) and light (>0.5)
    let light_muted = infos
        .iter()
        .filter(|c| c.saturation < 0.4 && c.lightness > 0.5)
        .min_by(|a, b| a.saturation.partial_cmp(&b.saturation).unwrap())
        .unwrap_or(muted);
    palette.insert(
        "light_muted".to_string(),
        format_hex(light_muted.rgb[0], light_muted.rgb[1], light_muted.rgb[2]),
    );

    // Dark muted: desaturated (<0.4) and dark (<0.5)
    let dark_muted = infos
        .iter()
        .filter(|c| c.saturation < 0.4 && c.lightness < 0.5)
        .min_by(|a, b| a.saturation.partial_cmp(&b.saturation).unwrap())
        .unwrap_or(muted);
    palette.insert(
        "dark_muted".to_string(),
        format_hex(dark_muted.rgb[0], dark_muted.rgb[1], dark_muted.rgb[2]),
    );

    palette
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_sl_pure_red() {
        let (s, l) = rgb_to_sl(255, 0, 0);
        assert!((s - 1.0).abs() < 0.01);
        assert!((l - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_rgb_to_sl_gray() {
        let (s, l) = rgb_to_sl(128, 128, 128);
        assert!(s.abs() < 0.01);
        assert!((l - 0.502).abs() < 0.01);
    }

    #[test]
    fn test_format_hex() {
        assert_eq!(format_hex(255, 0, 128), "#FF0080");
        assert_eq!(format_hex(0, 0, 0), "#000000");
    }

    #[test]
    fn test_average_color() {
        let pixels = vec![[255, 0, 0], [0, 255, 0], [0, 0, 255]];
        let avg = average_color(&pixels);
        assert_eq!(avg, [85, 85, 85]);
    }

    #[test]
    fn test_median_cut_single_color() {
        let pixels = vec![[255, 0, 0]; 100];
        let result = median_cut(pixels, 3);
        assert!(!result.is_empty());
        // All pixels are the same, so all buckets should average to red
        for (color, _count) in &result {
            assert_eq!(color, &[255, 0, 0]);
        }
    }

    #[test]
    fn test_categorize_colors_basic() {
        let colors = vec![
            ([255, 0, 0], 500),    // red - vibrant, dominant
            ([0, 0, 200], 300),    // blue - vibrant
            ([200, 200, 200], 200), // light gray - muted
            ([50, 50, 50], 100),   // dark gray - dark muted
            ([255, 200, 200], 80), // light pink - light muted
            ([0, 100, 0], 60),     // dark green - dark vibrant
            ([200, 255, 200], 40), // light green - light vibrant
            ([100, 50, 50], 20),   // dark red
        ];
        let palette = categorize_colors(&colors);

        assert_eq!(palette.get("dominant").unwrap(), "#FF0000");
        assert!(palette.contains_key("vibrant"));
        assert!(palette.contains_key("muted"));
        assert!(palette.contains_key("light_vibrant"));
        assert!(palette.contains_key("dark_vibrant"));
        assert!(palette.contains_key("light_muted"));
        assert!(palette.contains_key("dark_muted"));
    }

    #[test]
    fn test_categorize_empty() {
        let palette = categorize_colors(&[]);
        assert_eq!(palette.len(), 7);
        for (_, v) in &palette {
            assert_eq!(v, "#808080");
        }
    }
}
