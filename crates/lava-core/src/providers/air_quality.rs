use std::path::PathBuf;
use std::time::Duration;

use super::{DataProvider, ProviderData};

struct AirQualityConfig {
    api_key: String,
    lat: String,
    lon: String,
}

pub struct AirQualityProvider {
    interval_secs: u64,
}

impl AirQualityProvider {
    pub fn new() -> Self {
        Self {
            interval_secs: 600,
        }
    }
}

fn read_aq_config() -> Option<AirQualityConfig> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let path = config_dir.join("lava").join("settings.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    // Reuse the weather API key
    let weather = json.get("weather")?;
    let api_key = weather.get("apiKey")?.as_str().unwrap_or("").to_string();
    if api_key.is_empty() {
        return None;
    }

    // Air quality requires lat/lon coordinates from the weather location
    let location = weather
        .get("location")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let parts: Vec<&str> = location.split(',').map(|s| s.trim()).collect();
    if parts.len() == 2 {
        if let (Ok(_), Ok(_)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
            return Some(AirQualityConfig {
                api_key,
                lat: parts[0].to_string(),
                lon: parts[1].to_string(),
            });
        }
    }

    // If location is a city name, resolve coordinates via OWM geocoding
    if !location.is_empty() {
        let geo_url = format!(
            "https://api.openweathermap.org/geo/1.0/direct?q={}&limit=1&appid={}",
            location.replace(' ', "+"),
            api_key
        );
        if let Some(geo_json) = http_get_json(&geo_url) {
            if let Some(arr) = geo_json.as_array() {
                if let Some(first) = arr.first() {
                    let lat = first.get("lat").and_then(|v| v.as_f64());
                    let lon = first.get("lon").and_then(|v| v.as_f64());
                    if let (Some(lat), Some(lon)) = (lat, lon) {
                        return Some(AirQualityConfig {
                            api_key,
                            lat: lat.to_string(),
                            lon: lon.to_string(),
                        });
                    }
                }
            }
        }
    }

    eprintln!("[air_quality] Could not determine lat/lon from weather location");
    None
}

fn http_get_json(url: &str) -> Option<serde_json::Value> {
    match ureq::get(url).call() {
        Ok(resp) => resp.into_json().ok(),
        Err(e) => {
            eprintln!("[air_quality] HTTP error: {e}");
            None
        }
    }
}

fn aqi_to_label(aqi: u64) -> &'static str {
    match aqi {
        1 => "Good",
        2 => "Fair",
        3 => "Moderate",
        4 => "Poor",
        5 => "Very Poor",
        _ => "Unknown",
    }
}

fn fetch_air_quality(config: &AirQualityConfig) -> ProviderData {
    let mut data = ProviderData::new();

    let url = format!(
        "https://api.openweathermap.org/data/2.5/air_pollution?lat={}&lon={}&appid={}",
        config.lat, config.lon, config.api_key
    );

    let json = match http_get_json(&url) {
        Some(j) => j,
        None => return data,
    };

    let list = match json.get("list").and_then(|v| v.as_array()) {
        Some(l) => l,
        None => {
            eprintln!("[air_quality] Unexpected response format");
            return data;
        }
    };

    let entry = match list.first() {
        Some(e) => e,
        None => return data,
    };

    // AQI index
    if let Some(main) = entry.get("main") {
        if let Some(aqi) = main.get("aqi").and_then(|v| v.as_u64()) {
            data.insert("index".into(), aqi.to_string());
            data.insert("label".into(), aqi_to_label(aqi).to_string());
        }
    }

    // Component concentrations
    if let Some(components) = entry.get("components") {
        if let Some(v) = components.get("pm2_5").and_then(|v| v.as_f64()) {
            data.insert("pm25".into(), format!("{:.1}", v));
        }
        if let Some(v) = components.get("pm10").and_then(|v| v.as_f64()) {
            data.insert("pm10".into(), format!("{:.1}", v));
        }
        if let Some(v) = components.get("co").and_then(|v| v.as_f64()) {
            data.insert("co".into(), format!("{:.1}", v));
        }
        if let Some(v) = components.get("no2").and_then(|v| v.as_f64()) {
            data.insert("no2".into(), format!("{:.1}", v));
        }
        if let Some(v) = components.get("o3").and_then(|v| v.as_f64()) {
            data.insert("o3".into(), format!("{:.1}", v));
        }
        if let Some(v) = components.get("so2").and_then(|v| v.as_f64()) {
            data.insert("so2".into(), format!("{:.1}", v));
        }
        if let Some(v) = components.get("nh3").and_then(|v| v.as_f64()) {
            data.insert("nh3".into(), format!("{:.1}", v));
        }
        if let Some(v) = components.get("no").and_then(|v| v.as_f64()) {
            data.insert("no".into(), format!("{:.1}", v));
        }
    }

    data.insert(
        "updated".into(),
        chrono::Local::now().format("%H:%M").to_string(),
    );

    data
}

impl DataProvider for AirQualityProvider {
    fn prefix(&self) -> &str {
        "aq"
    }

    fn poll(&mut self) -> ProviderData {
        let config = match read_aq_config() {
            Some(c) => c,
            None => return ProviderData::new(),
        };

        fetch_air_quality(&config)
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_secs)
    }
}
