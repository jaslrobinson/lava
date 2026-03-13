use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use lava_core::pid;

pub static WALLPAPER_ACTIVE: AtomicBool = AtomicBool::new(false);
static WALLPAPER_PID: Mutex<Option<Child>> = Mutex::new(None);

const SHOW_EDITOR_SIGNAL: &str = "/tmp/lava-show-editor";

/// Start a background thread that watches for the show-editor signal file.
/// When the wallpaper process creates it, we show + focus the main window.
pub fn start_signal_watcher(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(250));
            if std::path::Path::new(SHOW_EDITOR_SIGNAL).exists() {
                let _ = std::fs::remove_file(SHOW_EDITOR_SIGNAL);
                if let Some(window) = app.get_webview_window("main") {
                    window.show().ok();
                    window.set_focus().ok();
                    eprintln!("[wallpaper] Signal received — showing editor window");
                }
            }
        }
    });
}

/// Check if wallpaper is currently running (editor-spawned or standalone).
/// If a standalone wallpaper is detected, adopt it by setting WALLPAPER_ACTIVE.
pub fn is_wallpaper_active() -> bool {
    if WALLPAPER_ACTIVE.load(Ordering::Relaxed) {
        return true;
    }
    if pid::is_wallpaper_running() {
        // Adopt the standalone wallpaper so stop_wallpaper_mode works
        WALLPAPER_ACTIVE.store(true, Ordering::Relaxed);
        return true;
    }
    false
}

/// Kill the wallpaper helper process if running (used by tray quit).
pub fn kill_wallpaper_process() {
    // Kill editor-spawned child process if we have one
    if let Some(mut child) = WALLPAPER_PID.lock().unwrap_or_else(|e| e.into_inner()).take() {
        eprintln!("[wallpaper] Killing helper process PID {} (tray quit)", child.id());
        let _ = child.kill();
        let _ = child.wait();
    }
    // Also kill standalone wallpaper if running
    if pid::is_wallpaper_running() {
        eprintln!("[wallpaper] Killing standalone wallpaper process");
        pid::kill_wallpaper();
    }
    WALLPAPER_ACTIVE.store(false, Ordering::Relaxed);
}

/// Find the lava-wallpaper binary (next to the main binary, or in target/debug)
fn find_wallpaper_binary() -> Option<std::path::PathBuf> {
    // Check next to the current executable
    if let Ok(exe) = std::env::current_exe() {
        let dir = exe.parent()?;
        let candidate = dir.join("lava-wallpaper");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    // Check workspace target/debug
    let candidates = [
        "target/debug/lava-wallpaper",
        "../target/debug/lava-wallpaper",
        "../../target/debug/lava-wallpaper",
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

    // Kill any standalone wallpaper before starting an editor-managed one
    if pid::is_wallpaper_running() {
        eprintln!("[wallpaper] Killing standalone wallpaper before starting editor-managed one");
        pid::kill_wallpaper();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let display_server = if std::env::var("WAYLAND_DISPLAY").is_ok() {
        "wayland"
    } else if std::env::var("DISPLAY").is_ok() {
        "x11"
    } else {
        return Err("No display server detected".into());
    };

    let compositor = detect_compositor();

    let binary = find_wallpaper_binary()
        .ok_or_else(|| "lava-wallpaper binary not found. Build it with: cargo build -p lava-wallpaper".to_string())?;

    // Launch wallpaper in standalone mode so it survives editor exit.
    // The frontend auto-saves before calling this, so lastProjectPath is current.
    // Always use production mode (own HTTP server) — never --dev, because Vite
    // dies with the editor and takes the wallpaper page down.
    // Use `setsid --fork` to fully detach from the editor's process group/session.
    let args = vec!["--fork", binary.to_str().unwrap_or("lava-wallpaper"), "--standalone"];

    eprintln!("[wallpaper] Spawning setsid --fork {:?} --standalone", binary);

    let child = Command::new("setsid")
        .args(&args)
        .spawn()
        .map_err(|e| format!("Failed to spawn wallpaper process: {}", e))?;

    // setsid --fork exits immediately; the real wallpaper PID is in /tmp/lava-wallpaper.pid
    // Wait briefly for the wallpaper to write its PID file
    eprintln!("[wallpaper] Standalone wallpaper launched via setsid");
    drop(child);

    // Register global shortcut (Super+Escape) to exit wallpaper mode
    let shortcut = Shortcut::new(Some(Modifiers::SUPER), Code::Escape);
    let app = window.app_handle().clone();
    let gs = app.global_shortcut();
    let _ = gs.unregister(shortcut);
    if let Err(e) = gs.on_shortcut(shortcut, move |handle, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            handle.emit("exit-wallpaper", ()).ok();
        }
    }) {
        eprintln!("[wallpaper] Shortcut registration failed (may already be registered): {}", e);
    }

    WALLPAPER_ACTIVE.store(true, Ordering::Relaxed);
    Ok(format!("{} ({})", display_server, compositor))
}

#[tauri::command]
pub fn stop_wallpaper_mode(window: tauri::WebviewWindow) -> Result<(), String> {
    if !WALLPAPER_ACTIVE.load(Ordering::Relaxed) && !pid::is_wallpaper_running() {
        return Err("Wallpaper mode is not active".into());
    }

    // Kill the wallpaper helper process
    if let Some(mut child) = WALLPAPER_PID.lock().unwrap_or_else(|e| e.into_inner()).take() {
        eprintln!("[wallpaper] Killing helper process PID {}", child.id());
        let _ = child.kill();
        let _ = child.wait();
    }

    // Also kill standalone wallpaper if running
    if pid::is_wallpaper_running() {
        eprintln!("[wallpaper] Killing standalone wallpaper process");
        pid::kill_wallpaper();
    }

    // Unregister global shortcut
    let shortcut = Shortcut::new(Some(Modifiers::SUPER), Code::Escape);
    window.app_handle().global_shortcut().unregister(shortcut).ok();

    WALLPAPER_ACTIVE.store(false, Ordering::Relaxed);
    Ok(())
}

/// Find the frontend dist directory.
/// In release builds, Tauri embeds the frontend, but we also need to serve it
/// via HTTP for the standalone wallpaper process. Look for the dist/ directory
/// next to the binary or in the resource path.
fn find_dist_dir() -> Option<std::path::PathBuf> {
    // Check next to the executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // Release install: dist/ next to the binary
            let candidate = dir.join("dist");
            if candidate.exists() && candidate.join("index.html").exists() {
                return Some(candidate);
            }
            // Tauri build output: ../dist relative to src-tauri target
            let candidate = dir.join("../dist");
            if candidate.exists() && candidate.join("index.html").exists() {
                return Some(candidate.canonicalize().ok()?);
            }
        }
    }
    // System install: /usr/share/lava/dist
    let system = std::path::PathBuf::from("/usr/share/lava/dist");
    if system.exists() && system.join("index.html").exists() {
        return Some(system);
    }
    // Development: check common relative paths
    for path in ["dist", "../dist", "../../dist"] {
        let p = std::path::PathBuf::from(path);
        if p.exists() && p.join("index.html").exists() {
            return p.canonicalize().ok();
        }
    }
    None
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

