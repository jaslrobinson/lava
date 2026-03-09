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
