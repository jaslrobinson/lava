use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{DataProvider, ProviderData};

struct WeatherConfig {
    api_key: String,
    location: String,
    units: String,
    source: String,
    interval_secs: Option<u64>,
}

/// Shared forecast data so both WeatherProvider and ForecastProvider can access it
type SharedForecast = Arc<Mutex<ProviderData>>;

pub struct WeatherProvider {
    interval_secs: u64,
    forecast_data: SharedForecast,
}

pub struct ForecastProvider {
    forecast_data: SharedForecast,
}

/// Create both providers together (they share forecast data)
pub fn create_providers() -> (WeatherProvider, ForecastProvider) {
    let shared = Arc::new(Mutex::new(ProviderData::new()));
    (
        WeatherProvider {
            interval_secs: 300,
            forecast_data: shared.clone(),
        },
        ForecastProvider {
            forecast_data: shared,
        },
    )
}

fn read_weather_config() -> Option<WeatherConfig> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let path = config_dir.join("kllw").join("settings.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    let weather = json.get("weather")?;
    let enabled = weather.get("enabled")?.as_bool().unwrap_or(false);
    if !enabled {
        return None;
    }

    let api_key = weather.get("apiKey")?.as_str().unwrap_or("").to_string();
    if api_key.is_empty() {
        return None;
    }

    let location = weather
        .get("location")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let units = weather
        .get("units")
        .and_then(|v| v.as_str())
        .unwrap_or("metric")
        .to_string();
    let source = weather
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or("openweathermap")
        .to_string();

    let interval_secs = json.get("providers")
        .and_then(|p| p.get("weather"))
        .and_then(|v| v.as_u64());

    Some(WeatherConfig {
        api_key,
        location,
        units,
        source,
        interval_secs,
    })
}

fn http_get_json(url: &str) -> Option<serde_json::Value> {
    match ureq::get(url).call() {
        Ok(resp) => resp.into_json().ok(),
        Err(e) => {
            eprintln!("[weather] HTTP error: {e}");
            None
        }
    }
}

/// Build OWM API URL, detecting lat/lon vs city name
fn build_owm_url(endpoint: &str, config: &WeatherConfig) -> String {
    let loc = config.location.trim();
    // Check if location looks like coordinates: "lat,lon" or "lat , lon"
    let parts: Vec<&str> = loc.split(',').map(|s| s.trim()).collect();
    let location_param = if parts.len() == 2 {
        if let (Ok(_lat), Ok(_lon)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
            format!("lat={}&lon={}", parts[0], parts[1])
        } else {
            format!("q={}", loc.replace(' ', "+"))
        }
    } else {
        format!("q={}", loc.replace(' ', "+"))
    };
    format!(
        "https://api.openweathermap.org/data/2.5/{endpoint}?{location_param}&units={}&appid={}",
        config.units, config.api_key
    )
}

fn fetch_openweathermap(config: &WeatherConfig) -> (ProviderData, ProviderData) {
    let mut current = ProviderData::new();
    let mut forecast = ProviderData::new();

    // --- Current weather ---
    let url = build_owm_url("weather", config);

    let json = match http_get_json(&url) {
        Some(j) => j,
        None => return (current, forecast),
    };

    // Check for API error
    if let Some(cod) = json.get("cod") {
        let code = cod
            .as_u64()
            .unwrap_or_else(|| cod.as_str().and_then(|s| s.parse().ok()).unwrap_or(200));
        if code != 200 {
            let msg = json
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            eprintln!("[weather] API error {code}: {msg}");
            return (current, forecast);
        }
    }

    // Temperature
    if let Some(main) = json.get("main") {
        if let Some(t) = main.get("temp").and_then(|v| v.as_f64()) {
            current.insert("temp".into(), format!("{:.0}", t));
            if config.units == "imperial" {
                current.insert("tempc".into(), format!("{:.0}", (t - 32.0) * 5.0 / 9.0));
            } else {
                current.insert("tempc".into(), format!("{:.0}", t));
            }
        }
        if let Some(fl) = main.get("feels_like").and_then(|v| v.as_f64()) {
            current.insert("flik".into(), format!("{:.0}", fl));
        }
        if let Some(h) = main.get("humidity").and_then(|v| v.as_f64()) {
            current.insert("hum".into(), format!("{:.0}", h));
        }
        if let Some(p) = main.get("pressure").and_then(|v| v.as_f64()) {
            current.insert("press".into(), format!("{:.0}", p));
        }
    }

    // Condition
    if let Some(weather_arr) = json.get("weather").and_then(|v| v.as_array()) {
        if let Some(w) = weather_arr.first() {
            if let Some(desc) = w.get("description").and_then(|v| v.as_str()) {
                current.insert("cond".into(), capitalize(desc));
            }
            if let Some(id) = w.get("id").and_then(|v| v.as_u64()) {
                current.insert("code".into(), id.to_string());
                current.insert("icon".into(), owm_code_to_icon(id));
            }
            // OWM icon code → image URL for use in Image layers
            if let Some(ic) = w.get("icon").and_then(|v| v.as_str()) {
                current.insert(
                    "iconurl".into(),
                    format!("https://openweathermap.org/img/wn/{ic}@4x.png"),
                );
            }
        }
    }

    // Wind
    if let Some(wind) = json.get("wind") {
        if let Some(speed) = wind.get("speed").and_then(|v| v.as_f64()) {
            current.insert("wspeed".into(), format!("{:.1}", speed));
            if config.units == "imperial" {
                current.insert("wspeedm".into(), format!("{:.1}", speed * 0.44704));
            } else {
                current.insert("wspeedm".into(), format!("{:.1}", speed));
            }
        }
        if let Some(deg) = wind.get("deg").and_then(|v| v.as_f64()) {
            current.insert("wdir".into(), deg_to_compass(deg));
        }
        if let Some(gust) = wind.get("gust").and_then(|v| v.as_f64()) {
            current.insert("wgust".into(), format!("{:.1}", gust));
        }
    }

    // Clouds
    if let Some(clouds) = json.get("clouds") {
        if let Some(all) = clouds.get("all").and_then(|v| v.as_f64()) {
            current.insert("clouds".into(), format!("{:.0}", all));
        }
    }

    // Dew point (computed from temp + humidity via Magnus formula)
    if let (Some(temp_s), Some(hum_s)) = (current.get("temp").cloned(), current.get("hum").cloned())
    {
        if let (Ok(t), Ok(h)) = (temp_s.parse::<f64>(), hum_s.parse::<f64>()) {
            let a = 17.27;
            let b = 237.7;
            let alpha = (a * t) / (b + t) + (h / 100.0).ln();
            let dew = (b * alpha) / (a - alpha);
            current.insert("dpoint".into(), format!("{:.0}", dew));
        }
    }

    // Visibility
    if let Some(vis) = json.get("visibility").and_then(|v| v.as_f64()) {
        current.insert("vis".into(), format!("{:.0}", vis / 1000.0));
    }

    current.insert("provider".into(), "openweathermap".into());
    current.insert(
        "updated".into(),
        chrono::Local::now().format("%H:%M").to_string(),
    );

    // --- 5-day forecast → stored under "wf" prefix with keys like "0_temp", "1_icon" ---
    fetch_owm_forecast(config, &mut forecast);

    (current, forecast)
}

fn fetch_owm_forecast(config: &WeatherConfig, forecast: &mut ProviderData) {
    let url = build_owm_url("forecast", config) + "&cnt=40";

    let json = match http_get_json(&url) {
        Some(j) => j,
        None => return,
    };

    let list = match json.get("list").and_then(|v| v.as_array()) {
        Some(l) => l,
        None => return,
    };

    // Group by day
    let mut days: HashMap<String, Vec<&serde_json::Value>> = HashMap::new();
    for entry in list {
        if let Some(dt_txt) = entry.get("dt_txt").and_then(|v| v.as_str()) {
            let date = dt_txt.split(' ').next().unwrap_or("");
            days.entry(date.to_string()).or_default().push(entry);
        }
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut sorted_days: Vec<String> = days.keys().cloned().collect();
    sorted_days.sort();

    let mut day_idx = 0u32;
    for day_str in &sorted_days {
        if day_idx >= 5 {
            break;
        }
        let entries = match days.get(day_str) {
            Some(e) => e,
            None => continue,
        };

        // Pick entry closest to noon
        let noon_entry = entries
            .iter()
            .min_by_key(|e| {
                e.get("dt_txt")
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        let hour: i32 = s
                            .split(' ')
                            .nth(1)
                            .unwrap_or("00")
                            .split(':')
                            .next()
                            .unwrap_or("0")
                            .parse()
                            .unwrap_or(0);
                        (hour - 12).unsigned_abs()
                    })
                    .unwrap_or(24)
            })
            .unwrap_or(&&serde_json::Value::Null);

        // Min/max across all entries
        let mut temp_min = f64::MAX;
        let mut temp_max = f64::MIN;
        for e in entries {
            if let Some(main) = e.get("main") {
                if let Some(tmin) = main.get("temp_min").and_then(|v| v.as_f64()) {
                    temp_min = temp_min.min(tmin);
                }
                if let Some(tmax) = main.get("temp_max").and_then(|v| v.as_f64()) {
                    temp_max = temp_max.max(tmax);
                }
            }
        }

        // Keys: "0_temp", "0_min", "0_max", "0_icon", etc.
        // Used as wf(0, temp) → evaluator joins args → "0_temp"
        let idx = day_idx.to_string();

        if temp_min < f64::MAX {
            forecast.insert(format!("{idx}_min"), format!("{:.0}", temp_min));
        }
        if temp_max > f64::MIN {
            forecast.insert(format!("{idx}_max"), format!("{:.0}", temp_max));
        }

        if let Some(main) = noon_entry.get("main") {
            if let Some(t) = main.get("temp").and_then(|v| v.as_f64()) {
                forecast.insert(format!("{idx}_temp"), format!("{:.0}", t));
            }
            if let Some(h) = main.get("humidity").and_then(|v| v.as_f64()) {
                forecast.insert(format!("{idx}_hum"), format!("{:.0}", h));
            }
        }

        if let Some(weather_arr) = noon_entry.get("weather").and_then(|v| v.as_array()) {
            if let Some(w) = weather_arr.first() {
                if let Some(desc) = w.get("description").and_then(|v| v.as_str()) {
                    forecast.insert(format!("{idx}_cond"), capitalize(desc));
                }
                if let Some(id) = w.get("id").and_then(|v| v.as_u64()) {
                    forecast.insert(format!("{idx}_icon"), owm_code_to_icon(id));
                    forecast.insert(format!("{idx}_code"), id.to_string());
                }
                if let Some(ic) = w.get("icon").and_then(|v| v.as_str()) {
                    forecast.insert(
                        format!("{idx}_iconurl"),
                        format!("https://openweathermap.org/img/wn/{ic}@4x.png"),
                    );
                }
            }
        }

        if let Some(wind) = noon_entry.get("wind") {
            if let Some(speed) = wind.get("speed").and_then(|v| v.as_f64()) {
                forecast.insert(format!("{idx}_wspeed"), format!("{:.1}", speed));
            }
        }

        // Day name
        if day_str == &today {
            forecast.insert(format!("{idx}_day"), "Today".into());
        } else if let Ok(date) = chrono::NaiveDate::parse_from_str(day_str, "%Y-%m-%d") {
            forecast.insert(format!("{idx}_day"), date.format("%a").to_string());
        }

        day_idx += 1;
    }
}

fn owm_code_to_icon(code: u64) -> String {
    let icon = match code {
        200..=232 => "THUNDERSTORM",
        300..=321 => "DRIZZLE",
        500..=504 => "RAIN",
        511 => "SLEET",
        520..=531 => "SHOWERS",
        600..=622 => "SNOW",
        701 => "MIST",
        711 => "SMOKE",
        721 => "HAZE",
        731 | 761 => "DUST",
        741 => "FOG",
        751 => "SAND",
        762 => "ASH",
        771 => "SQUALL",
        781 => "TORNADO",
        800 => "CLEAR",
        801 => "MOSTLY_CLEAR",
        802 => "PARTLY_CLOUDY",
        803 => "MOSTLY_CLOUDY",
        804 => "CLOUDY",
        _ => "UNKNOWN",
    };
    icon.to_string()
}

fn deg_to_compass(deg: f64) -> String {
    let dirs = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];
    let idx = ((deg + 11.25) / 22.5) as usize % 16;
    dirs[idx].to_string()
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

impl DataProvider for WeatherProvider {
    fn prefix(&self) -> &str {
        "wi"
    }

    fn poll(&mut self) -> ProviderData {
        let config = match read_weather_config() {
            Some(c) => c,
            None => return ProviderData::new(),
        };

        // Update interval from settings (extracted during config read)
        if let Some(interval) = config.interval_secs {
            if interval > 0 {
                self.interval_secs = interval;
            }
        }

        let (current, forecast_data) = match config.source.as_str() {
            "openweathermap" => fetch_openweathermap(&config),
            _ => {
                eprintln!("[weather] Unsupported source: {}", config.source);
                return ProviderData::new();
            }
        };

        // Store forecast data for the ForecastProvider to return
        if let Ok(mut fd) = self.forecast_data.lock() {
            *fd = forecast_data;
        }

        current
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_secs)
    }
}

impl DataProvider for ForecastProvider {
    fn prefix(&self) -> &str {
        "wf"
    }

    fn poll(&mut self) -> ProviderData {
        // Return the forecast data that WeatherProvider populated
        self.forecast_data
            .lock()
            .map(|d| d.clone())
            .unwrap_or_default()
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(300)
    }
}
