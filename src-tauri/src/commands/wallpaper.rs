use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

pub static WALLPAPER_ACTIVE: AtomicBool = AtomicBool::new(false);
static WALLPAPER_PID: Mutex<Option<u32>> = Mutex::new(None);

/// Check if wallpaper is currently running.
pub fn is_wallpaper_active() -> bool {
    WALLPAPER_ACTIVE.load(Ordering::Relaxed)
}

/// Kill the wallpaper helper process if running (used by tray quit).
pub fn kill_wallpaper_process() {
    if let Some(pid) = WALLPAPER_PID.lock().unwrap().take() {
        eprintln!("[wallpaper] Killing helper process PID {} (tray quit)", pid);
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
    }
    WALLPAPER_ACTIVE.store(false, Ordering::Relaxed);
}

/// Find the klwp-wallpaper binary (next to the main binary, or in target/debug)
fn find_wallpaper_binary() -> Option<std::path::PathBuf> {
    // Check next to the current executable
    if let Ok(exe) = std::env::current_exe() {
        let dir = exe.parent()?;
        let candidate = dir.join("klwp-wallpaper");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    // Check workspace target/debug
    let candidates = [
        "target/debug/klwp-wallpaper",
        "../target/debug/klwp-wallpaper",
        "../../target/debug/klwp-wallpaper",
    ];
    for c in candidates {
        let p = std::path::PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

#[tauri::command]
pub fn start_wallpaper_mode(window: tauri::WebviewWindow, project: serde_json::Value) -> Result<String, String> {
    if WALLPAPER_ACTIVE.load(Ordering::Relaxed) {
        return Err("Wallpaper mode is already active".into());
    }

    let display_server = if std::env::var("WAYLAND_DISPLAY").is_ok() {
        "wayland"
    } else if std::env::var("DISPLAY").is_ok() {
        "x11"
    } else {
        return Err("No display server detected".into());
    };

    let compositor = detect_compositor();

    // Get the URL for the wallpaper view
    let base_url = window.url().map_err(|e| e.to_string())?;
    let base_str = base_url.as_str().trim_end_matches('/');
    // If the URL uses tauri:// protocol, fall back to the dev server URL
    let wallpaper_url = if base_str.starts_with("http") {
        format!("{}?wallpaper=true", base_str)
    } else {
        "http://localhost:1420?wallpaper=true".to_string()
    };
    eprintln!("[wallpaper] Window URL: {}, wallpaper URL: {}", base_str, wallpaper_url);

    // Save project to temp file for the helper to load
    let project_path = std::env::temp_dir().join("klwp-wallpaper-project.json");
    let project_json = serde_json::to_string(&project).map_err(|e| e.to_string())?;
    std::fs::write(&project_path, &project_json).map_err(|e| e.to_string())?;

    let binary = find_wallpaper_binary()
        .ok_or_else(|| "klwp-wallpaper binary not found. Build it with: cargo build -p klwp-wallpaper".to_string())?;

    eprintln!("[wallpaper] Spawning {:?} with URL: {}", binary, wallpaper_url);

    let child = Command::new(&binary)
        .arg(&wallpaper_url)
        .arg(project_path.to_str().unwrap_or("/tmp/klwp-wallpaper-project.json"))
        .spawn()
        .map_err(|e| format!("Failed to spawn wallpaper process: {}", e))?;

    let pid = child.id();
    eprintln!("[wallpaper] Helper process started with PID {}", pid);
    *WALLPAPER_PID.lock().unwrap() = Some(pid);

    // Register global shortcut (Super+Escape) to exit wallpaper mode
    let shortcut = Shortcut::new(Some(Modifiers::SUPER), Code::Escape);
    let app = window.app_handle().clone();
    app.global_shortcut().on_shortcut(shortcut, move |handle, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            handle.emit("exit-wallpaper", ()).ok();
        }
    }).map_err(|e| format!("Failed to register shortcut: {}", e))?;

    WALLPAPER_ACTIVE.store(true, Ordering::Relaxed);
    Ok(format!("{} ({})", display_server, compositor))
}

#[tauri::command]
pub fn stop_wallpaper_mode(window: tauri::WebviewWindow) -> Result<(), String> {
    if !WALLPAPER_ACTIVE.load(Ordering::Relaxed) {
        return Err("Wallpaper mode is not active".into());
    }

    // Kill the wallpaper helper process
    if let Some(pid) = WALLPAPER_PID.lock().unwrap().take() {
        eprintln!("[wallpaper] Killing helper process PID {}", pid);
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
    }

    // Unregister global shortcut
    let shortcut = Shortcut::new(Some(Modifiers::SUPER), Code::Escape);
    window.app_handle().global_shortcut().unregister(shortcut).ok();

    WALLPAPER_ACTIVE.store(false, Ordering::Relaxed);
    Ok(())
}

fn detect_compositor() -> String {
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return "hyprland".to_string();
    }
    if std::env::var("SWAYSOCK").is_ok() {
        return "sway".to_string();
    }
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        return desktop.to_lowercase();
    }
    "unknown".to_string()
}

