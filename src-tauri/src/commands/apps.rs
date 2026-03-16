use serde::Serialize;
use std::path::PathBuf;

/// Resolve a .desktop Icon= value to an absolute file path.
/// Searches hicolor theme dirs and pixmaps in priority order.
pub fn resolve_icon_path(icon: &str) -> String {
    if icon.is_empty() { return String::new(); }
    // Already an absolute path
    if icon.starts_with('/') {
        if std::path::Path::new(icon).exists() { return icon.to_string(); }
        return String::new();
    }
    let sizes = ["256x256", "128x128", "64x64", "48x48", "32x32", "scalable"];
    let exts = ["png", "svg", "xpm"];
    // Theme dirs to search
    let mut theme_dirs: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/icons/hicolor"),
        PathBuf::from("/usr/share/icons/Papirus"),
        PathBuf::from("/usr/share/icons/Adwaita"),
        PathBuf::from("/usr/share/icons/breeze"),
    ];
    // Add user icon theme dirs
    if let Some(home) = dirs::home_dir() {
        theme_dirs.insert(0, home.join(".local/share/icons/hicolor"));
    }
    for theme in &theme_dirs {
        for size in &sizes {
            for ext in &exts {
                let p = theme.join(size).join("apps").join(format!("{}.{}", icon, ext));
                if p.exists() { return p.to_string_lossy().into_owned(); }
            }
        }
    }
    // Fallback: pixmaps
    for ext in &exts {
        let p = PathBuf::from(format!("/usr/share/pixmaps/{}.{}", icon, ext));
        if p.exists() { return p.to_string_lossy().into_owned(); }
    }
    String::new()
}

#[tauri::command]
pub fn resolve_icon(icon_name: String) -> String {
    resolve_icon_path(&icon_name)
}

#[derive(Debug, Serialize, Clone)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub categories: String,
}

#[tauri::command]
pub fn list_apps() -> Vec<AppEntry> {
    let mut dirs: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/applications"));
    }

    let mut apps: Vec<AppEntry> = Vec::new();
    for dir in dirs {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            if let Some(app) = parse_desktop_file(&path) {
                apps.push(app);
            }
        }
    }

    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps.dedup_by(|a, b| a.name == b.name);
    apps
}

fn parse_desktop_file(path: &PathBuf) -> Option<AppEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut name = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut categories = String::new();
    let mut no_display = false;
    let mut is_app = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();
        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if line.starts_with('[') {
            in_desktop_entry = false;
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if let Some(v) = line.strip_prefix("Name=") {
            if name.is_empty() { name = v.to_string(); }
        } else if let Some(v) = line.strip_prefix("Exec=") {
            exec = v.to_string();
        } else if let Some(v) = line.strip_prefix("Icon=") {
            icon = v.to_string();
        } else if let Some(v) = line.strip_prefix("Categories=") {
            categories = v.to_string();
        } else if line == "NoDisplay=true" {
            no_display = true;
        } else if line == "Type=Application" {
            is_app = true;
        }
    }

    if !is_app || no_display || name.is_empty() || exec.is_empty() {
        return None;
    }

    // Strip field codes (%f, %F, %u, %U, etc.) from exec
    let exec_clean = exec
        .split_whitespace()
        .filter(|p| !p.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ");

    let icon_resolved = resolve_icon_path(&icon);
    Some(AppEntry { name, exec: exec_clean, icon: icon_resolved, categories })
}

#[derive(Debug, Serialize, Clone)]
pub struct WindowState {
    pub running_classes: Vec<String>,
    pub active_class: String,
}

#[tauri::command]
pub fn get_window_state() -> WindowState {
    let mut running_classes: Vec<String> = Vec::new();
    let mut active_class = String::new();

    // Get all open clients
    if let Ok(output) = std::process::Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
    {
        if let Ok(json) = serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout) {
            for client in &json {
                if let Some(class) = client["class"].as_str() {
                    if !class.is_empty() {
                        running_classes.push(class.to_lowercase());
                    }
                }
            }
        }
    }

    // Get active window class
    if let Ok(output) = std::process::Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
    {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if let Some(class) = json["class"].as_str() {
                active_class = class.to_lowercase();
            }
        }
    }

    running_classes.dedup();
    WindowState { running_classes, active_class }
}
