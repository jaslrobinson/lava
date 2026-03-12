use std::path::PathBuf;

fn settings_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = config_dir.join("lava");
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

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let autostart_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("autostart");
    std::fs::create_dir_all(&autostart_dir).map_err(|e| e.to_string())?;

    let desktop_path = autostart_dir.join("lava.desktop");

    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=LAVA\nComment=Live Animated Visuals for Arch\nExec={}\nIcon=lava\nX-GNOME-Autostart-enabled=true\nStartupNotify=false\n",
            exe.display()
        );
        std::fs::write(&desktop_path, content).map_err(|e| e.to_string())?;
    } else {
        if desktop_path.exists() {
            std::fs::remove_file(&desktop_path).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
