use std::process::Command;
use std::time::Duration;

use super::{DataProvider, ProviderData};

pub struct SysInfoProvider {
    hostname: String,
    kernel: String,
    distro: String,
    distro_version: String,
}

impl SysInfoProvider {
    pub fn new() -> Self {
        let hostname = std::fs::read_to_string("/etc/hostname")
            .unwrap_or_else(|_| "unknown".into())
            .trim()
            .to_string();

        let kernel = std::fs::read_to_string("/proc/version")
            .unwrap_or_default()
            .split_whitespace()
            .nth(2)
            .unwrap_or("unknown")
            .to_string();

        let (distro, version) = Self::read_os_release();

        Self {
            hostname,
            kernel,
            distro,
            distro_version: version,
        }
    }

    fn read_os_release() -> (String, String) {
        let content = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        let mut name = "Linux".to_string();
        let mut version = String::new();
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("NAME=") {
                name = val.trim_matches('"').to_string();
            } else if let Some(val) = line.strip_prefix("VERSION_ID=") {
                version = val.trim_matches('"').to_string();
            }
        }
        (name, version)
    }
}

impl DataProvider for SysInfoProvider {
    fn prefix(&self) -> &str {
        "si"
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();
        data.insert("model".into(), self.hostname.clone());
        data.insert("man".into(), self.distro.clone());
        data.insert("build".into(), self.kernel.clone());
        data.insert("aver".into(), self.distro_version.clone());

        // Uptime from /proc/uptime
        if let Ok(uptime_str) = std::fs::read_to_string("/proc/uptime") {
            if let Some(secs_str) = uptime_str.split_whitespace().next() {
                if let Ok(secs) = secs_str.parse::<f64>() {
                    let hours = (secs / 3600.0) as u64;
                    let mins = ((secs % 3600.0) / 60.0) as u64;
                    data.insert("boot".into(), format!("{}h {}m", hours, mins));
                    data.insert("uptime".into(), format!("{:.0}", secs));
                }
            }
        }

        // Dark mode detection - default to dark on Linux
        data.insert("darkmode".into(), "1".into());

        // Volume - try wpctl (PipeWire) first
        if let Ok(output) = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
        {
            if output.status.success() {
                let vol_str = String::from_utf8_lossy(&output.stdout);
                // Output: "Volume: 0.50" or "Volume: 0.50 [MUTED]"
                if let Some(vol_part) = vol_str.split_whitespace().nth(1) {
                    if let Ok(vol) = vol_part.parse::<f64>() {
                        data.insert("volr".into(), format!("{}", (vol * 100.0) as i32));
                        data.insert("vola".into(), format!("{}", (vol * 100.0) as i32));
                    }
                }
                let muted = vol_str.contains("MUTED");
                data.insert(
                    "ringer".into(),
                    if muted { "SILENT" } else { "NORMAL" }.into(),
                );
            }
        }

        data
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(5)
    }
}
