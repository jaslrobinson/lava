use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

use super::DataProvider;

/// Shared provider data: prefix -> field -> value
pub type SharedProviderData = Arc<RwLock<HashMap<String, HashMap<String, String>>>>;

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
    pub fn start(self, app: AppHandle) {
        let data = self.data;
        let mut providers = self.providers;

        tauri::async_runtime::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(1));

            loop {
                tick.tick().await;
                let now = Instant::now();
                let mut changed = false;

                for entry in providers.iter_mut() {
                    if now.duration_since(entry.last_poll) >= entry.interval {
                        let new_data = entry.provider.poll();
                        let prefix = entry.provider.prefix().to_string();
                        let mut data_write = data.write().await;
                        let current = data_write.entry(prefix).or_default();
                        if *current != new_data {
                            *current = new_data;
                            changed = true;
                        }
                        entry.last_poll = now;
                    }
                }

                if changed {
                    let data_read = data.read().await;
                    let _ = app.emit("provider-data-update", &*data_read);
                }
            }
        });
    }

    pub fn data(&self) -> SharedProviderData {
        self.data.clone()
    }
}
