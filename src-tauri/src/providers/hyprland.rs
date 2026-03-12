use std::process::Command;
use std::time::Duration;

use super::{DataProvider, ProviderData};

pub struct HyprlandProvider;

impl HyprlandProvider {
    pub fn new() -> Self {
        Self
    }

    fn read_gpu_usage() -> Option<String> {
        // AMD: sysfs gpu_busy_percent
        for card in ["card0", "card1"] {
            let path = format!("/sys/class/drm/{}/device/gpu_busy_percent", card);
            if let Ok(val) = std::fs::read_to_string(&path) {
                return Some(val.trim().to_string());
            }
        }
        // NVIDIA fallback
        if let Ok(output) = Command::new("nvidia-smi")
            .args(["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
            .output()
        {
            if output.status.success() {
                return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }
        None
    }
}

impl DataProvider for HyprlandProvider {
    fn prefix(&self) -> &str {
        "hy"
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();

        // Active workspace
        if let Ok(output) = Command::new("hyprctl")
            .args(["activeworkspace", "-j"])
            .output()
        {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                let id = json["id"].as_i64().unwrap_or(1);
                data.insert("workspace".into(), id.to_string());
                data.insert(
                    "workspace_name".into(),
                    json["name"].as_str().unwrap_or("").to_string(),
                );
                data.insert(
                    "workspace_windows".into(),
                    json["windows"].as_i64().unwrap_or(0).to_string(),
                );
            }
        }

        let active_ws = data
            .get("workspace")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(1);

        // All workspaces
        if let Ok(output) = Command::new("hyprctl")
            .args(["workspaces", "-j"])
            .output()
        {
            if let Ok(workspaces) =
                serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout)
            {
                for ws in &workspaces {
                    let id = ws["id"].as_i64().unwrap_or(0);
                    if id < 1 {
                        continue;
                    }
                    let windows = ws["windows"].as_i64().unwrap_or(0);
                    data.insert(format!("ws_{}_windows", id), windows.to_string());
                    data.insert(format!("ws_{}_exists", id), "1".to_string());
                }
            }
        }

        // Fill in workspace 1-10 active/exists flags
        for i in 1..=10 {
            if !data.contains_key(&format!("ws_{}_exists", i)) {
                data.insert(format!("ws_{}_exists", i), "0".to_string());
                data.insert(format!("ws_{}_windows", i), "0".to_string());
            }
            data.insert(
                format!("ws_{}_active", i),
                if i == active_ws {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
            );
        }

        // Clients (running windows)
        if let Ok(output) = Command::new("hyprctl")
            .args(["clients", "-j"])
            .output()
        {
            if let Ok(clients) =
                serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout)
            {
                data.insert("app_count".into(), clients.len().to_string());

                let mut classes: Vec<String> = clients
                    .iter()
                    .filter_map(|c| {
                        c["class"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                    })
                    .collect();
                classes.sort();
                classes.dedup();
                data.insert("apps".into(), classes.join(", "));
                data.insert("app_list_count".into(), classes.len().to_string());

                // Focused window
                if let Some(focused) = clients.iter().find(|c| c["focusHistoryID"].as_i64() == Some(0)) {
                    data.insert(
                        "focused_app".into(),
                        focused["class"].as_str().unwrap_or("").to_string(),
                    );
                    data.insert(
                        "focused_title".into(),
                        focused["title"].as_str().unwrap_or("").to_string(),
                    );
                }
            }
        }

        // GPU usage
        if let Some(gpu) = Self::read_gpu_usage() {
            data.insert("gpu".into(), gpu);
        } else {
            data.insert("gpu".into(), "0".into());
        }

        data
    }

    fn interval(&self) -> Duration {
        Duration::from_millis(500)
    }
}
