use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lava");
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load_settings() -> Result<serde_json::Value, String> {
    let path = settings_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(serde_json::json!({}))
    }
}

pub fn last_project_path() -> Option<String> {
    let settings = load_settings().ok()?;
    settings.get("lastProjectPath")?.as_str().map(|s| s.to_string())
}

/// Find the frontend dist directory for serving to wallpaper WebKitGTK.
pub fn find_dist_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("dist");
            if candidate.exists() && candidate.join("index.html").exists() {
                return Some(candidate);
            }
            let candidate = dir.join("../dist");
            if candidate.exists() && candidate.join("index.html").exists() {
                return Some(candidate.canonicalize().ok()?);
            }
        }
    }
    let system = PathBuf::from("/usr/share/lava/dist");
    if system.exists() && system.join("index.html").exists() {
        return Some(system);
    }
    for path in ["dist", "../dist", "../../dist"] {
        let p = PathBuf::from(path);
        if p.exists() && p.join("index.html").exists() {
            return p.canonicalize().ok();
        }
    }
    None
}
