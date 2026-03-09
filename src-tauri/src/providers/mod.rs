pub mod battery;
pub mod datetime;
pub mod manager;
pub mod music;
pub mod network;
pub mod resource_monitor;
pub mod sysinfo_provider;
pub mod traffic;

use std::collections::HashMap;
use std::time::Duration;

/// All providers store their data as string key-value pairs.
/// The formula engine converts to appropriate types.
pub type ProviderData = HashMap<String, String>;

pub trait DataProvider: Send + Sync {
    /// Unique prefix (e.g., "bi", "wi", "mi")
    fn prefix(&self) -> &str;

    /// Poll for new data. Returns the current key-value pairs.
    fn poll(&mut self) -> ProviderData;

    /// How often to poll
    fn interval(&self) -> Duration;
}
