use super::wallpaper;

#[tauri::command]
pub fn hide_editor(window: tauri::WebviewWindow) -> Result<(), String> {
    window
        .hide()
        .map_err(|e| format!("Failed to hide window: {}", e))
}

#[tauri::command]
pub fn show_editor(window: tauri::WebviewWindow) -> Result<(), String> {
    window
        .show()
        .map_err(|e| format!("Failed to show window: {}", e))?;
    window
        .set_focus()
        .map_err(|e| format!("Failed to focus window: {}", e))
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    // Kill wallpaper process if running
    wallpaper::kill_wallpaper_process();
    // Exit the app
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn is_wallpaper_running() -> bool {
    wallpaper::is_wallpaper_active()
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .map_err(|e| format!("Failed to open URL: {}", e))?;
    Ok(())
}

/// Control music playback via playerctl.
/// action: "play", "pause", "play-pause", "next", "previous", "stop"
#[tauri::command]
pub fn music_control(action: String) -> Result<(), String> {
    let allowed = ["play", "pause", "play-pause", "next", "previous", "stop"];
    if !allowed.contains(&action.as_str()) {
        return Err(format!("Unknown music action: {}", action));
    }
    std::process::Command::new("playerctl")
        .arg(&action)
        .spawn()
        .map_err(|e| format!("playerctl failed: {}", e))?;
    Ok(())
}

/// Adjust system volume via wpctl (WirePlumber).
/// delta: percentage to adjust, e.g. 5 for +5%, -5 for -5%
#[tauri::command]
pub fn adjust_volume(delta: i32) -> Result<(), String> {
    // Use relative percentage — no blocking read needed
    let arg = if delta >= 0 {
        format!("{}%+", delta)
    } else {
        format!("{}%-", -delta)
    };
    std::process::Command::new("wpctl")
        .args(["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", &arg])
        .spawn()
        .map_err(|e| format!("wpctl set-volume failed: {}", e))?;
    Ok(())
}

/// Launch an application by command string.
#[tauri::command]
pub fn launch_app(command: String) -> Result<(), String> {
    // Split on whitespace for simple commands with args
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".into());
    }
    std::process::Command::new(parts[0])
        .args(&parts[1..])
        .spawn()
        .map_err(|e| format!("Failed to launch '{}': {}", command, e))?;
    Ok(())
}
