use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize, Clone)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub categories: String,
}

pub fn resolve_icon_path(icon: &str) -> String {
    if icon.is_empty() { return String::new(); }
    if icon.starts_with('/') {
        if std::path::Path::new(icon).exists() { return icon.to_string(); }
        return String::new();
    }
    let sizes = ["256x256", "128x128", "64x64", "48x48", "32x32", "scalable"];
    let exts = ["png", "svg", "xpm"];
    let mut theme_dirs: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/icons/hicolor"),
        PathBuf::from("/usr/share/icons/Papirus"),
        PathBuf::from("/usr/share/icons/Adwaita"),
        PathBuf::from("/usr/share/icons/breeze"),
    ];
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
    for ext in &exts {
        let p = PathBuf::from(format!("/usr/share/pixmaps/{}.{}", icon, ext));
        if p.exists() { return p.to_string_lossy().into_owned(); }
    }
    String::new()
}

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
        if !in_desktop_entry { continue; }
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

    let exec_clean = exec
        .split_whitespace()
        .filter(|p| !p.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ");

    let icon_resolved = resolve_icon_path(&icon);
    Some(AppEntry { name, exec: exec_clean, icon: icon_resolved, categories })
}
