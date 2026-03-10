use std::time::Duration;

use super::{DataProvider, ProviderData};

pub struct RadarProvider;

/// Convert lat/lon to Slippy Map tile coordinates at a given zoom level
fn latlon_to_tile(lat: f64, lon: f64, zoom: u32) -> (u32, u32) {
    let n = 2_u32.pow(zoom) as f64;
    let x = ((lon + 180.0) / 360.0 * n).floor() as u32;
    let lat_rad = lat.to_radians();
    let y = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n).floor() as u32;
    (x, y)
}

impl DataProvider for RadarProvider {
    fn prefix(&self) -> &str {
        "wm"
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();

        // Read lat/lon from weather config
        let (lat, lon) = match read_location() {
            Some(loc) => loc,
            None => {
                eprintln!("[radar] No location configured (need lat,lon format)");
                return data;
            }
        };

        // Compute tile coordinates at zoom 6 (regional view, ~300km, good centering)
        let zoom = 6_u32;
        let (tx, ty) = latlon_to_tile(lat, lon, zoom);

        // Base map: CartoDB dark_all (has roads, cities, county boundaries)
        let map_url = format!(
            "https://basemaps.cartocdn.com/dark_all/{zoom}/{tx}/{ty}@2x.png"
        );
        data.insert("mapurl".into(), map_url);

        // Fetch RainViewer radar frame list (no API key needed)
        let json = match ureq::get("https://api.rainviewer.com/public/weather-maps.json").call() {
            Ok(resp) => match resp.into_json::<serde_json::Value>() {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("[radar] JSON parse error: {e}");
                    return data;
                }
            },
            Err(e) => {
                eprintln!("[radar] HTTP error: {e}");
                return data;
            }
        };

        // Extract host and latest radar frame path
        let host = json
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("https://tilecache.rainviewer.com");

        let path = json
            .get("radar")
            .and_then(|r| r.get("past"))
            .and_then(|p| p.as_array())
            .and_then(|arr| arr.last())
            .and_then(|frame| frame.get("path"))
            .and_then(|p| p.as_str())
            .unwrap_or("");

        if path.is_empty() {
            eprintln!("[radar] No radar frames available");
            return data;
        }

        // RainViewer radar overlay tile (transparent PNG)
        // Format: {host}{path}/{size}/{z}/{x}/{y}/{color}/{smooth}_{snow}.png
        // size=512, color=2 (blue), smooth=1, snow=1
        let radar_url = format!(
            "{host}{path}/512/{zoom}/{tx}/{ty}/2/1_1.png"
        );

        data.insert("radarurl".into(), radar_url);

        data
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(300) // 5 minutes
    }
}

fn read_location() -> Option<(f64, f64)> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let path = config_dir.join("kllw").join("settings.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    let weather = json.get("weather")?;
    let location = weather.get("location")?.as_str().unwrap_or("");

    let parts: Vec<&str> = location.split(',').map(|s| s.trim()).collect();
    if parts.len() == 2 {
        let lat = parts[0].parse::<f64>().ok()?;
        let lon = parts[1].parse::<f64>().ok()?;
        Some((lat, lon))
    } else {
        None
    }
}
