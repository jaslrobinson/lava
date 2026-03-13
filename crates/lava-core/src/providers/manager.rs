use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use super::{DataProvider, SharedProviderData};

pub struct ProviderHandle {
    shutdown: Arc<AtomicBool>,
}

impl ProviderHandle {
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
            last_poll: Instant::now() - Duration::from_secs(3600),
        });
    }

    pub fn data(&self) -> SharedProviderData {
        self.data.clone()
    }

    /// Start the provider loop in a background thread.
    /// `on_update` is called whenever data changes (with the full data snapshot and a bool
    /// indicating whether anything changed this tick).
    /// If `on_update` is None, only temp files are written.
    pub fn start<F>(self, on_update: Option<F>) -> ProviderHandle
    where
        F: Fn(&HashMap<String, HashMap<String, String>>, bool) + Send + 'static,
    {
        let data = self.data;
        let mut providers = self.providers;
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = shutdown.clone();

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(250));
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }
                let now = Instant::now();
                let mut changed = false;

                for entry in providers.iter_mut() {
                    if now.duration_since(entry.last_poll) >= entry.interval {
                        let new_data = entry.provider.poll();
                        let prefix = entry.provider.prefix().to_string();
                        let mut data_write = data.write().unwrap();
                        let current = data_write.entry(prefix).or_default();
                        for (k, v) in &new_data {
                            if current.get(k) != Some(v) {
                                current.insert(k.clone(), v.clone());
                                changed = true;
                            }
                        }
                        entry.last_poll = now;
                    }
                }

                // Write provider data to temp file + call callback
                {
                    let data_read = data.read().unwrap();
                    if let Some(ref callback) = on_update {
                        callback(&data_read, changed);
                    }
                    if let Ok(json) = serde_json::to_string(&*data_read) {
                        let path = std::env::temp_dir().join("lava-provider-data.json");
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
}

pub fn cleanup_temp_files() {
    let path = std::env::temp_dir().join("lava-provider-data.json");
    let _ = std::fs::remove_file(&path);
    let tmp_path = std::env::temp_dir().join("lava-provider-data.json.tmp");
    let _ = std::fs::remove_file(&tmp_path);
    let proj_path = std::env::temp_dir().join("lava-wallpaper-project.json");
    let _ = std::fs::remove_file(&proj_path);
}
