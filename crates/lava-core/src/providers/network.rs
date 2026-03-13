use std::process::Command;
use std::time::Duration;

use super::{DataProvider, ProviderData};

pub struct NetworkProvider;

impl DataProvider for NetworkProvider {
    fn prefix(&self) -> &str {
        "nc"
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();
        data.insert("connected".into(), "0".into());

        // Try nmcli for network info
        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "TYPE,STATE,CONNECTION,DEVICE", "device", "status"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 3 && parts[1] == "connected" {
                        data.insert("connected".into(), "1".into());
                        data.insert("type".into(), parts[0].to_lowercase());
                        data.insert("name".into(), parts[2].to_string());
                        break;
                    }
                }
            }
        }

        // Get WiFi SSID and signal
        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "ACTIVE,SSID,SIGNAL", "dev", "wifi"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 3 && parts[0] == "yes" {
                        data.insert("ssid".into(), parts[1].to_string());
                        data.insert("strength".into(), parts[2].to_string());
                    }
                }
            }
        }

        // Get IP address
        if let Ok(output) = Command::new("hostname").args(["-I"]).output() {
            if output.status.success() {
                let ip = String::from_utf8_lossy(&output.stdout);
                if let Some(first_ip) = ip.split_whitespace().next() {
                    data.insert("ip".into(), first_ip.to_string());
                }
            }
        }

        data
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(5)
    }
}
