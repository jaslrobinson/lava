use std::process::Command;
use std::time::Duration;

use super::{DataProvider, ProviderData};

pub struct MusicProvider;

impl DataProvider for MusicProvider {
    fn prefix(&self) -> &str {
        "mi"
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();

        // Try playerctl for MPRIS data
        let queries = [
            ("title", "{{title}}"),
            ("artist", "{{artist}}"),
            ("album", "{{album}}"),
            ("status", "{{status}}"),
            ("volume", "{{volume}}"),
            ("position", "{{position}}"),
            ("length", "{{mpris:length}}"),
            ("cover", "{{mpris:artUrl}}"),
        ];

        for (field, fmt) in &queries {
            if let Ok(output) = Command::new("playerctl")
                .args(["metadata", "--format", fmt])
                .output()
            {
                if output.status.success() {
                    let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !val.is_empty() {
                        data.insert(field.to_string(), val);
                    }
                }
            }
        }

        // Map status to KLWP format
        if let Some(status) = data.get("status") {
            let klwp_state = match status.as_str() {
                "Playing" => "PLAYING",
                "Paused" => "PAUSED",
                _ => "STOPPED",
            };
            data.insert("state".into(), klwp_state.into());
        } else {
            data.insert("state".into(), "STOPPED".into());
        }

        // Convert position/length from microseconds to seconds
        if let Some(pos_str) = data.get("position").cloned() {
            if let Ok(pos_us) = pos_str.parse::<u64>() {
                let pos_secs = pos_us / 1_000_000;
                data.insert("pos".into(), pos_secs.to_string());
            }
        }
        if let Some(len_str) = data.get("length").cloned() {
            if let Ok(len_us) = len_str.parse::<u64>() {
                let len_secs = len_us / 1_000_000;
                data.insert("len".into(), len_secs.to_string());

                // Calculate percent
                if let Some(pos_str) = data.get("pos") {
                    if let Ok(pos) = pos_str.parse::<f64>() {
                        let pct = if len_secs > 0 {
                            (pos / len_secs as f64) * 100.0
                        } else {
                            0.0
                        };
                        data.insert("percent".into(), format!("{:.0}", pct));
                    }
                }
            }
        }

        // Volume as 0-100
        if let Some(vol_str) = data.get("volume").cloned() {
            if let Ok(vol) = vol_str.parse::<f64>() {
                data.insert("vol".into(), format!("{}", (vol * 100.0) as i32));
            }
        }

        // Player name
        if let Ok(output) = Command::new("playerctl").args(["--list-all"]).output() {
            if output.status.success() {
                let players = String::from_utf8_lossy(&output.stdout);
                if let Some(first) = players.lines().next() {
                    data.insert("package".into(), first.trim().to_string());
                }
            }
        }

        data
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(1)
    }
}
