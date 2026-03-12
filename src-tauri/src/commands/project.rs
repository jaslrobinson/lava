use crate::klwp_import::{self, KompImportResult};
use crate::project::Project;

use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;

#[tauri::command]
pub fn save_project(path: String, project: Project) -> Result<(), String> {
    println!("[RUST] save_project called for path: {}", path);
    println!("[RUST] Project has {} layers", project.layers.len());

    // Count animations
    let anim_count: usize = project
        .layers
        .iter()
        .map(|l| l.animations.as_ref().map(|a| a.len()).unwrap_or(0))
        .sum();
    println!("[RUST] Total animations in project: {}", anim_count);

    let json = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;

    println!("[RUST] JSON size: {} bytes", json.len());
    println!("[RUST] Writing to file...");

    fs::write(&path, json).map_err(|e| {
        let err_msg = format!("Failed to write file {}: {}", path, e);
        println!("[RUST] {}", err_msg);
        err_msg
    })?;

    println!("[RUST] Save successful!");
    Ok(())
}

#[tauri::command]
pub fn load_project(path: String) -> Result<Project, String> {
    println!("[RUST] load_project called for path: {}", path);

    let json = fs::read_to_string(&path).map_err(|e| {
        let err_msg = format!("Failed to read file {}: {}", path, e);
        println!("[RUST] {}", err_msg);
        err_msg
    })?;

    println!("[RUST] JSON size: {} bytes", json.len());

    let project: Project = serde_json::from_str(&json).map_err(|e| {
        let err_msg = format!("Failed to parse JSON from {}: {}", path, e);
        println!("[RUST] {}", err_msg);
        err_msg
    })?;

    println!(
        "[RUST] Loaded project '{}' with {} layers",
        project.name,
        project.layers.len()
    );

    // Count animations
    let anim_count: usize = project
        .layers
        .iter()
        .map(|l| l.animations.as_ref().map(|a| a.len()).unwrap_or(0))
        .sum();
    println!("[RUST] Total animations loaded: {}", anim_count);

    // Log first few layers with their animation counts
    for (i, layer) in project.layers.iter().take(3).enumerate() {
        let anim_count = layer.animations.as_ref().map(|a| a.len()).unwrap_or(0);
        println!(
            "[RUST] Layer {} '{}': {} animations",
            i, layer.name, anim_count
        );
        if let Some(anims) = layer.animations.as_ref() {
            if anims.len() > 0 {
                println!(
                    "[RUST]   First animation: type={:?}, trigger={:?}",
                    anims[0].animation_type, anims[0].trigger
                );
            }
        }
    }

    Ok(project)
}

#[tauri::command]
pub fn import_komp(path: String) -> Result<KompImportResult, String> {
    let komp_path = Path::new(&path);
    let stem = komp_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported_komp");
    let output_dir = komp_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("{}_assets", stem));

    fs::create_dir_all(&output_dir).map_err(|e| format!("Cannot create output dir: {}", e))?;

    // Peek inside the ZIP to detect native vs KLWP format
    let native_json = read_native_preset_json(&path);

    if let Some(json) = native_json {
        let file2 = fs::File::open(&path).map_err(|e| format!("Cannot open file: {}", e))?;
        let mut archive2 = zip::ZipArchive::new(file2).map_err(|e| format!("Not a valid .komp ZIP: {}", e))?;
        return import_native_komp(&json, output_dir.to_str().unwrap_or("."), &mut archive2);
    }

    // Fall back to KLWP importer
    klwp_import::import_komp_file(&path, output_dir.to_str().unwrap_or("."), 1920, 1080)
}

/// Try to read preset.json from a ZIP and return its contents if it looks like a native format.
/// Returns None if the file doesn't exist, can't be read, or is not a native format.
fn read_native_preset_json(path: &str) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let json = {
        let mut entry = archive.by_name("preset.json").ok()?;
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut entry, &mut buf).ok()?;
        buf
    };
    let value = serde_json::from_str::<serde_json::Value>(&json).ok()?;
    if value.get("version").is_some() && value.get("layers").is_some() {
        Some(json)
    } else {
        None
    }
}

fn import_native_komp(
    json: &str,
    output_dir: &str,
    archive: &mut zip::ZipArchive<fs::File>,
) -> Result<KompImportResult, String> {
    use crate::project::Layer;

    let project: crate::project::Project = serde_json::from_str(json)
        .map_err(|e| format!("Failed to parse native .komp: {}", e))?;

    // Extract assets to output_dir
    let mut asset_count = 0;
    for i in 0..archive.len() {
        if let Ok(mut entry) = archive.by_index(i) {
            let name = entry.name().to_string();
            if name.starts_with("assets/") && !name.ends_with('/') {
                let rel = &name["assets/".len()..];
                let dest = Path::new(output_dir).join(rel);
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Ok(mut out) = fs::File::create(&dest) {
                    let _ = std::io::copy(&mut entry, &mut out);
                    asset_count += 1;
                }
            }
        }
    }

    // Build a synthetic root overlap layer from the project layers
    let root_layer = if project.layers.len() == 1 {
        project.layers.into_iter().next().unwrap()
    } else {
        Layer {
            id: format!("komp_{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()),
            name: project.name.clone(),
            layer_type: crate::project::LayerType::Overlap,
            properties: crate::project::LayerProperties {
                x: crate::project::NumberOrString::Number(0.0),
                y: crate::project::NumberOrString::Number(0.0),
                width: crate::project::NumberOrString::Number(project.resolution.width as f64),
                height: crate::project::NumberOrString::Number(project.resolution.height as f64),
                rotation: None,
                scale_x: None,
                scale_y: None,
                opacity: None,
                anchor: None,
                visible: None,
                text: None,
                font_size: None,
                font_family: None,
                color: None,
                text_align: None,
                max_lines: None,
                line_spacing: None,
                shadow: None,
                shape_kind: None,
                fill: None,
                stroke: None,
                stroke_width: None,
                corner_radius: None,
                skew_x: None,
                skew_y: None,
                src: None,
                scale_mode: None,
                tint: None,
                style: None,
                min: None,
                max: None,
                value: None,
                track_color: None,
                icon_set: None,
                glyph_code: None,
                orientation: None,
                spacing: None,
                click_action: None,
                scroll_action: None,
                icon_src: None,
                viz_style: None,
                bar_count: None,
                bar_spacing: None,
                sensitivity: None,
                color_top: None,
                color_mid: None,
                color_bottom: None,
                peak_color: None,
            },
            animations: None,
            children: Some(project.layers),
            locked: None,
            visible: None,
        }
    };

    Ok(KompImportResult {
        root: root_layer,
        globals: project.globals,
        warnings: vec![],
        asset_count,
        asset_dir: output_dir.to_string(),
    })
}

/// List font files in a project's asset directory
#[tauri::command]
pub fn list_project_fonts(asset_dir: String) -> Result<Vec<String>, String> {
    let fonts_dir = Path::new(&asset_dir).join("fonts");
    if !fonts_dir.exists() {
        return Ok(vec![]);
    }
    let mut fonts = Vec::new();
    let entries =
        fs::read_dir(&fonts_dir).map_err(|e| format!("Failed to read fonts dir: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "ttf" | "otf" | "woff" | "woff2" => {
                    fonts.push(path.to_string_lossy().to_string());
                }
                _ => {}
            }
        }
    }
    Ok(fonts)
}

/// Copy a file into the project's asset directory under a subfolder
#[tauri::command]
pub fn copy_asset_to_project(
    source_path: String,
    asset_dir: String,
    subfolder: String,
) -> Result<String, String> {
    let target_dir = Path::new(&asset_dir).join(&subfolder);
    fs::create_dir_all(&target_dir).map_err(|e| format!("Failed to create dir: {}", e))?;

    let filename = Path::new(&source_path)
        .file_name()
        .ok_or("Invalid source path")?
        .to_string_lossy()
        .to_string();

    let target_path = target_dir.join(&filename);
    fs::copy(&source_path, &target_path).map_err(|e| format!("Failed to copy file: {}", e))?;

    Ok(target_path.to_string_lossy().to_string())
}

/// Write an icon file (SVG content) to the project's icons directory
#[tauri::command]
pub fn write_icon_file(
    asset_dir: String,
    filename: String,
    content: String,
) -> Result<String, String> {
    let icons_dir = std::path::Path::new(&asset_dir).join("icons");
    std::fs::create_dir_all(&icons_dir)
        .map_err(|e| format!("Failed to create icons dir: {}", e))?;

    let file_path = icons_dir.join(&filename);
    std::fs::write(&file_path, &content)
        .map_err(|e| format!("Failed to write icon file: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

/// Extract the launcher icon from an APK file
#[tauri::command]
pub fn extract_apk_icon(apk_path: String, asset_dir: String) -> Result<String, String> {
    let file = fs::File::open(&apk_path).map_err(|e| format!("Failed to open APK: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read APK as ZIP: {}", e))?;

    // Search for launcher icon in priority order
    let icon_patterns = [
        "res/mipmap-xxxhdpi/ic_launcher.png",
        "res/mipmap-xxxhdpi/ic_launcher_round.png",
        "res/mipmap-xxhdpi/ic_launcher.png",
        "res/mipmap-xxhdpi/ic_launcher_round.png",
        "res/mipmap-xhdpi/ic_launcher.png",
        "res/drawable-xxxhdpi/ic_launcher.png",
        "res/drawable-xxhdpi/ic_launcher.png",
    ];

    let mut found_name: Option<String> = None;

    // Try exact matches first
    for pattern in &icon_patterns {
        if archive.by_name(pattern).is_ok() {
            found_name = Some(pattern.to_string());
            break;
        }
    }

    // Fallback: search for any ic_launcher PNG
    if found_name.is_none() {
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                let name = entry.name().to_string();
                if name.contains("ic_launcher") && name.ends_with(".png") && !name.contains("xml") {
                    found_name = Some(name);
                    break;
                }
            }
        }
    }

    let icon_name = found_name.ok_or("No launcher icon found in APK")?;

    // Extract the icon
    let icons_dir = Path::new(&asset_dir).join("icons");
    fs::create_dir_all(&icons_dir).map_err(|e| format!("Failed to create icons dir: {}", e))?;

    // Name the output after the APK
    let apk_stem = Path::new(&apk_path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let output_path = icons_dir.join(format!("{}.png", apk_stem));

    let mut icon_file = archive
        .by_name(&icon_name)
        .map_err(|e| format!("Failed to read icon from APK: {}", e))?;
    let mut output = fs::File::create(&output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    io::copy(&mut icon_file, &mut output).map_err(|e| format!("Failed to write icon: {}", e))?;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_komp(path: String, project: Project) -> Result<(), String> {
    println!("[RUST] export_komp: saving to {}", path);

    let json = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;

    let file = fs::File::create(&path)
        .map_err(|e| format!("Cannot create .komp file: {}", e))?;

    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("preset.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(json.as_bytes())
        .map_err(|e| e.to_string())?;

    // Bundle assets if the project has an asset directory
    if let Some(ref asset_dir) = project.asset_dir {
        let asset_path = Path::new(asset_dir.as_str());
        if asset_path.exists() {
            add_dir_to_zip(&mut zip, asset_path, "assets")
                .unwrap_or_else(|e| println!("[RUST] Warning: could not bundle assets: {}", e));
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    println!("[RUST] export_komp: done");
    Ok(())
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<fs::File>,
    dir: &Path,
    prefix: &str,
) -> Result<(), String> {
    let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    fn walk(
        zip: &mut zip::ZipWriter<fs::File>,
        dir: &Path,
        base: &Path,
        prefix: &str,
        options: zip::write::FileOptions<()>,
    ) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let relative = path.strip_prefix(base).map_err(|e| e.to_string())?;
            let zip_path = format!("{}/{}", prefix, relative.to_string_lossy().replace('\\', "/"));
            if path.is_dir() {
                walk(zip, &path, base, prefix, options)?;
            } else {
                zip.start_file(&zip_path, options).map_err(|e| e.to_string())?;
                let data = fs::read(&path).map_err(|e| e.to_string())?;
                zip.write_all(&data).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    walk(zip, dir, dir, prefix, options)
}
