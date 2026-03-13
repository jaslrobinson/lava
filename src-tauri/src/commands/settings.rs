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
        // Find the wallpaper binary for autostart (lightweight, no editor needed)
        let wallpaper_bin = find_wallpaper_binary_for_autostart();
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=LAVA Wallpaper\nComment=Live Animated Visuals for Arch\nExec={} --standalone\nIcon=lava\nX-GNOME-Autostart-enabled=true\nStartupNotify=false\n",
            wallpaper_bin
        );
        std::fs::write(&desktop_path, content).map_err(|e| e.to_string())?;
    } else {
        if desktop_path.exists() {
            std::fs::remove_file(&desktop_path).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

fn find_wallpaper_binary_for_autostart() -> String {
    // Check next to the current executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("lava-wallpaper");
            if candidate.exists() {
                return candidate.display().to_string();
            }
        }
    }
    // System paths
    for path in ["/usr/bin/lava-wallpaper", "/usr/local/bin/lava-wallpaper"] {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    // Fallback
    "lava-wallpaper".to_string()
}
