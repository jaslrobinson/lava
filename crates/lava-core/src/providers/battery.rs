use std::path::PathBuf;
use std::time::Duration;

use super::{DataProvider, ProviderData};

pub struct BatteryProvider {
    battery_path: Option<PathBuf>,
}

impl BatteryProvider {
    pub fn new() -> Self {
        let path = std::fs::read_dir("/sys/class/power_supply/")
            .ok()
            .and_then(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .find(|e| e.file_name().to_string_lossy().starts_with("BAT"))
                    .map(|e| e.path())
            });
        Self { battery_path: path }
    }
}

impl DataProvider for BatteryProvider {
    fn prefix(&self) -> &str {
        "bi"
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();
        let Some(path) = &self.battery_path else {
            data.insert("level".into(), "100".into());
            data.insert("status".into(), "FULL".into());
            data.insert("plugged".into(), "1".into());
            return data;
        };

        // Read capacity (0-100)
        if let Ok(cap) = std::fs::read_to_string(path.join("capacity")) {
            data.insert("level".into(), cap.trim().into());
        }

        // Read status
        if let Ok(status) = std::fs::read_to_string(path.join("status")) {
            let s = status.trim().to_uppercase();
            let klwp_status = match s.as_str() {
                "CHARGING" => "CHARGING",
                "DISCHARGING" => "DISCHARGING",
                "NOT CHARGING" | "NOT_CHARGING" => "DISCHARGING",
                "FULL" => "FULL",
                _ => &s,
            };
            data.insert("status".into(), klwp_status.into());
        }

        // Read temp if available (in tenths of degree C)
        if let Ok(temp) = std::fs::read_to_string(path.join("temp")) {
            if let Ok(t) = temp.trim().parse::<f64>() {
                data.insert("temp".into(), format!("{:.1}", t / 10.0));
            }
        }

        // Plugged status
        let plugged = data
            .get("status")
            .map(|s| s != "DISCHARGING")
            .unwrap_or(false);
        data.insert("plugged".into(), if plugged { "1" } else { "0" }.into());

        data
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(10)
    }
}
