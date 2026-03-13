use std::path::Path;

const PID_PATH: &str = "/tmp/lava-wallpaper.pid";
const LOCK_PATH: &str = "/tmp/lava-provider-master.lock";

pub fn write_pid() {
    let _ = std::fs::write(PID_PATH, std::process::id().to_string());
}

pub fn read_pid() -> Option<u32> {
    std::fs::read_to_string(PID_PATH).ok()?.trim().parse().ok()
}

pub fn cleanup_pid() {
    let _ = std::fs::remove_file(PID_PATH);
}

pub fn is_wallpaper_running() -> bool {
    if let Some(pid) = read_pid() {
        Path::new(&format!("/proc/{}", pid)).exists()
    } else {
        false
    }
}

pub fn kill_wallpaper() -> bool {
    if let Some(pid) = read_pid() {
        if Path::new(&format!("/proc/{}", pid)).exists() {
            kill_process(pid);
            cleanup_pid();
            return true;
        }
    }
    false
}

fn kill_process(pid: u32) {
    use std::process::Command;
    let _ = Command::new("kill").arg(pid.to_string()).status();
}

pub fn try_acquire_provider_lock() -> bool {
    let my_pid = std::process::id().to_string();
    // Check if lock exists and PID is alive
    if let Ok(content) = std::fs::read_to_string(LOCK_PATH) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            if Path::new(&format!("/proc/{}", pid)).exists() {
                return false; // Another process holds the lock
            }
        }
    }
    // Acquire lock
    std::fs::write(LOCK_PATH, &my_pid).is_ok()
}

pub fn release_provider_lock() {
    // Only release if we own it
    let my_pid = std::process::id().to_string();
    if let Ok(content) = std::fs::read_to_string(LOCK_PATH) {
        if content.trim() == my_pid {
            let _ = std::fs::remove_file(LOCK_PATH);
        }
    }
}
