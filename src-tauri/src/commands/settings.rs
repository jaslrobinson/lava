use std::path::PathBuf;

fn settings_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = config_dir.join("kllw");
    std::fs::create_dir_all(&dir).ok();
    dir.join("settings.json")
}

#[tauri::command]
pub fn load_settings() -> Result<String, String> {
    let path = settings_path();
    if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| e.to_string())
    } else {
        Ok("{}".into())
    }
}

#[tauri::command]
pub fn save_settings(data: String) -> Result<(), String> {
    let path = settings_path();
    std::fs::write(&path, data).map_err(|e| e.to_string())
}
