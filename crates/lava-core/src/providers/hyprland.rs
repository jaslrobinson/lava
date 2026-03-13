use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::time::Duration;

use super::{DataProvider, ProviderData, SharedProviderData};

/// In-memory workspace state, updated directly from socket events.
struct WsState {
    active: i64,
    /// workspace_id -> window count (only tracked workspaces)
    workspaces: HashMap<i64, i64>,
    active_window_class: String,
}

impl WsState {
    fn new() -> Self {
        Self {
            active: 1,
            workspaces: HashMap::new(),
            active_window_class: String::new(),
        }
    }

    /// Bootstrap from hyprctl (called once at startup)
    fn init_from_hyprctl(&mut self) {
        if let Ok(output) = Command::new("hyprctl").args(["activeworkspace", "-j"]).output() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                self.active = json["id"].as_i64().unwrap_or(1);
            }
        }
        if let Ok(output) = Command::new("hyprctl").args(["workspaces", "-j"]).output() {
            if let Ok(wss) = serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout) {
                for ws in &wss {
                    let id = ws["id"].as_i64().unwrap_or(0);
                    if id >= 1 {
                        self.workspaces.insert(id, ws["windows"].as_i64().unwrap_or(0));
                    }
                }
            }
        }
    }

    /// Apply a socket2 event line, return true if state changed
    fn apply_event(&mut self, line: &str) -> bool {
        if let Some(id_str) = line.strip_prefix("workspace>>") {
            if let Ok(id) = id_str.trim().parse::<i64>() {
                self.active = id;
                self.workspaces.entry(id).or_insert(0);
                return true;
            }
        } else if let Some(id_str) = line.strip_prefix("createworkspace>>") {
            if let Ok(id) = id_str.trim().parse::<i64>() {
                if id >= 1 {
                    self.workspaces.entry(id).or_insert(0);
                    return true;
                }
            }
        } else if let Some(id_str) = line.strip_prefix("destroyworkspace>>") {
            if let Ok(id) = id_str.trim().parse::<i64>() {
                self.workspaces.remove(&id);
                return true;
            }
        } else if line.starts_with("openwindow>>") {
            // openwindow>>ADDR,WORKSPACE,CLASS,TITLE
            let parts: Vec<&str> = line["openwindow>>".len()..].splitn(4, ',').collect();
            if parts.len() >= 2 {
                if let Ok(ws_id) = parts[1].trim().parse::<i64>() {
                    if ws_id >= 1 {
                        *self.workspaces.entry(ws_id).or_insert(0) += 1;
                        return true;
                    }
                }
            }
        } else if line.starts_with("activewindow>>") {
            // activewindow>>CLASS,TITLE
            let payload = &line["activewindow>>".len()..];
            let class = payload.split(',').next().unwrap_or("").trim().to_string();
            self.active_window_class = class;
            return true;
        } else if line.starts_with("closewindow>>") {
            // Window closed -- we don't know which workspace, decrement active
            // (hyprctl clients will correct on next poll)
            return true;
        } else if line.starts_with("movewindow>>") {
            // movewindow>>ADDR,WORKSPACE
            return true;
        }
        false
    }

    /// Build ProviderData from current state
    fn to_provider_data(&self) -> ProviderData {
        let mut data = ProviderData::new();
        data.insert("workspace".into(), self.active.to_string());
        data.insert("workspace_name".into(), self.active.to_string());

        for i in 1..=10i64 {
            let exists = self.workspaces.contains_key(&i);
            let windows = self.workspaces.get(&i).copied().unwrap_or(0);
            data.insert(format!("ws_{}_exists", i), if exists { "1" } else { "0" }.into());
            data.insert(format!("ws_{}_windows", i), windows.to_string());
            data.insert(
                format!("ws_{}_active", i),
                if i == self.active { "1" } else { "0" }.into(),
            );
        }

        data
    }
}

pub struct HyprlandProvider;

impl HyprlandProvider {
    pub fn new() -> Self {
        Self
    }

    /// Start the event listener. Must be called after providers are registered
    /// so we have access to SharedProviderData.
    /// `on_update` is called whenever workspace state changes (with the full data snapshot).
    pub fn start_event_listener(
        shared: SharedProviderData,
        on_update: Option<Box<dyn Fn(&HashMap<String, HashMap<String, String>>) + Send>>,
    ) {
        std::thread::spawn(move || {
            if let Err(e) = run_event_loop(shared, on_update) {
                eprintln!("[hyprland] Event listener failed: {}", e);
            }
        });
    }

    fn read_gpu_usage() -> Option<String> {
        for card in ["card0", "card1"] {
            let path = format!("/sys/class/drm/{}/device/gpu_busy_percent", card);
            if let Ok(val) = std::fs::read_to_string(&path) {
                return Some(val.trim().to_string());
            }
        }
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

/// Write workspace data into shared provider data, call update callback, and write temp file.
fn flush_ws_data(
    shared: &SharedProviderData,
    ws_data: &ProviderData,
    on_update: &Option<Box<dyn Fn(&HashMap<String, HashMap<String, String>>) + Send>>,
) {
    let mut data = shared.write().unwrap();
    let hy = data.entry("hy".into()).or_default();
    for (k, v) in ws_data {
        hy.insert(k.clone(), v.clone());
    }
    if let Some(ref callback) = on_update {
        callback(&data);
    }
    // Write temp file for wallpaper process (atomic rename)
    if let Ok(json) = serde_json::to_string(&*data) {
        let tmp = std::env::temp_dir().join("lava-provider-data.json.tmp");
        let dst = std::env::temp_dir().join("lava-provider-data.json");
        if std::fs::write(&tmp, &json).is_ok() {
            let _ = std::fs::rename(&tmp, &dst);
        }
    }
}

/// Write target opacity to /tmp/lava-wallpaper-opacity based on active window.
fn write_opacity_signal(active_class: &str) {
    // Read fade settings from config
    let (enabled, opacity) = read_fade_settings();

    let target = if !enabled || active_class.is_empty() || active_class == "lava-wallpaper" {
        1.0
    } else {
        opacity
    };

    let tmp = std::env::temp_dir().join("lava-wallpaper-opacity.tmp");
    let dst = std::env::temp_dir().join("lava-wallpaper-opacity");
    if std::fs::write(&tmp, format!("{:.2}", target)).is_ok() {
        let _ = std::fs::rename(&tmp, &dst);
    }
}

/// Read fade settings from ~/.config/lava/settings.json (cached-friendly)
fn read_fade_settings() -> (bool, f64) {
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("lava")
        .join("settings.json");

    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            let enabled = json.get("wallpaperFadeEnabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let opacity = json.get("wallpaperFadeOpacity")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.3);
            return (enabled, opacity);
        }
    }
    (true, 0.3)
}

/// Event loop: listens on socket2, updates SharedProviderData directly, calls callback.
fn run_event_loop(
    shared: SharedProviderData,
    on_update: Option<Box<dyn Fn(&HashMap<String, HashMap<String, String>>) + Send>>,
) -> Result<(), String> {
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .map_err(|_| "HYPRLAND_INSTANCE_SIGNATURE not set".to_string())?;
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map_err(|_| "XDG_RUNTIME_DIR not set".to_string())?;
    let socket_path = format!("{}/hypr/{}/.socket2.sock", runtime_dir, sig);

    let mut state = WsState::new();
    state.init_from_hyprctl();
    flush_ws_data(&shared, &state.to_provider_data(), &on_update);
    write_opacity_signal(&state.active_window_class);

    eprintln!("[hyprland] Connecting to event socket: {}", socket_path);
    let stream = UnixStream::connect(&socket_path)
        .map_err(|e| format!("Failed to connect to socket2: {}", e))?;

    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if state.apply_event(&line) {
            flush_ws_data(&shared, &state.to_provider_data(), &on_update);
            // Write opacity signal when active window changes
            write_opacity_signal(&state.active_window_class);
        }
    }

    Err("Event socket closed".into())
}

impl DataProvider for HyprlandProvider {
    fn prefix(&self) -> &str {
        "hy"
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();

        // Clients (running windows) -- only thing that still needs polling
        if let Ok(output) = Command::new("hyprctl").args(["clients", "-j"]).output() {
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
        Duration::from_secs(1)
    }
}
