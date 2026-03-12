use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

use super::{DataProvider, SharedProviderData};

/// Handle to stop the provider polling loop.
#[allow(dead_code)]
pub struct ProviderHandle {
    shutdown: Arc<AtomicBool>,
}

#[allow(dead_code)]
impl ProviderHandle {
    /// Signal the provider loop to stop.
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

pub struct ProviderManager {
    providers: Vec<ProviderEntry>,
    data: SharedProviderData,
}

struct ProviderEntry {
    provider: Box<dyn DataProvider>,
    interval: Duration,
    last_poll: Instant,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&mut self, provider: Box<dyn DataProvider>) {
        let interval = provider.interval();
        self.providers.push(ProviderEntry {
            provider,
            interval,
            last_poll: Instant::now() - Duration::from_secs(3600), // force initial poll
        });
    }

    /// Start the provider loop. This should be called from Tauri's setup hook.
    /// Returns a handle that can be used to stop the loop on app exit.
    pub fn start(self, app: AppHandle) -> ProviderHandle {
        let data = self.data;
        let mut providers = self.providers;
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = shutdown.clone();

        tauri::async_runtime::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(250));

            loop {
                tick.tick().await;
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }
                let now = Instant::now();
                let mut changed = false;

                for entry in providers.iter_mut() {
                    if now.duration_since(entry.last_poll) >= entry.interval {
                        let new_data = tokio::task::block_in_place(|| entry.provider.poll());
                        let prefix = entry.provider.prefix().to_string();
                        let mut data_write = data.write().await;
                        let current = data_write.entry(prefix).or_default();
                        // Merge new data into existing (preserves keys from event listeners)
                        for (k, v) in &new_data {
                            if current.get(k) != Some(v) {
                                current.insert(k.clone(), v.clone());
                                changed = true;
                            }
                        }
                        entry.last_poll = now;
                    }
                }

                // Always write provider data to temp file for wallpaper WebKitGTK view
                {
                    let data_read = data.read().await;
                    if changed {
                        let _ = app.emit("provider-data-update", &*data_read);
                    }
                    if let Ok(json) = serde_json::to_string(&*data_read) {
                        let path = std::env::temp_dir().join("lava-provider-data.json");
                        // Write atomically: write to temp then rename to avoid partial reads
                        let tmp_path = std::env::temp_dir().join("lava-provider-data.json.tmp");
                        if std::fs::write(&tmp_path, &json).is_ok() {
                            let _ = std::fs::rename(&tmp_path, &path);
                        }
                    }
                }
            }
            cleanup_temp_files();
        });

        ProviderHandle { shutdown }
    }

    pub fn data(&self) -> SharedProviderData {
        self.data.clone()
    }
}

/// Clean up temp files used by provider data sharing.
pub fn cleanup_temp_files() {
    let path = std::env::temp_dir().join("lava-provider-data.json");
    let _ = std::fs::remove_file(&path);
    let tmp_path = std::env::temp_dir().join("lava-provider-data.json.tmp");
    let _ = std::fs::remove_file(&tmp_path);
    let proj_path = std::env::temp_dir().join("lava-wallpaper-project.json");
    let _ = std::fs::remove_file(&proj_path);
}
