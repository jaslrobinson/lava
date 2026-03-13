// Plugin system for LAVA
//
// Plugin manifests live in: ~/.config/lava/plugins/
// Each plugin is a JSON file describing a data provider.
//
// Example: ~/.config/lava/plugins/thunderbird-calendar.json
// {
//   "name": "Thunderbird Calendar",
//   "plugin_type": "calendar-ics",
//   "prefix": "ca",
//   "config": { "glob": "~/.thunderbird/*/calendar-data/**/*.ics" }
// }
//
// Provided formulas (prefix "ca"):
//   $ca(today_count)$       -- number of events today
//   $ca(next_title)$        -- next upcoming event title
//   $ca(next_time)$         -- next event start time (HH:MM)
//   $ca(next_date)$         -- next event date (e.g. "Mon Mar 10")
//   $ca(next_location)$     -- next event location
//   $ca(event0_title)$      -- 1st upcoming event title (0-4)
//   $ca(event0_time)$       -- 1st upcoming event time
//   $ca(event0_date)$       -- 1st upcoming event date
//   $ca(event0_location)$   -- 1st upcoming event location
//   $ca(event_count)$       -- total upcoming events (max 5)

pub mod calendar_ics;
pub mod calendar_sqlite;

use serde::Deserialize;
use std::path::PathBuf;
use crate::providers::DataProvider;

#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub plugin_type: String,
    pub prefix: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

pub fn plugins_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lava")
        .join("plugins")
}

/// Load all *.json plugin manifests from the plugins directory.
/// Malformed or unknown manifests are skipped with a warning.
pub fn load_plugins() -> Vec<Box<dyn DataProvider>> {
    let dir = plugins_dir();
    let mut providers: Vec<Box<dyn DataProvider>> = Vec::new();

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return providers, // plugins dir doesn't exist yet -- fine
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[plugins] Failed to read {:?}: {}", path, e);
                continue;
            }
        };
        let manifest: PluginManifest = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[plugins] Failed to parse {:?}: {}", path, e);
                continue;
            }
        };
        match manifest.plugin_type.as_str() {
            "calendar-ics" => {
                let glob = manifest
                    .config
                    .get("glob")
                    .and_then(|v| v.as_str())
                    .unwrap_or("~/.thunderbird/*/calendar-data/**/*.ics")
                    .to_string();
                eprintln!(
                    "[plugins] Loaded '{}' (prefix={}, glob={})",
                    manifest.name, manifest.prefix, glob
                );
                providers.push(Box::new(calendar_ics::CalendarIcsProvider::new(
                    manifest.prefix,
                    glob,
                )));
            }
            "calendar-thunderbird" => {
                let db_path = manifest
                    .config
                    .get("db_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("~/.thunderbird/*/calendar-data/cache.sqlite")
                    .to_string();
                eprintln!(
                    "[plugins] Loaded '{}' (prefix={}, db={})",
                    manifest.name, manifest.prefix, db_path
                );
                providers.push(Box::new(calendar_sqlite::CalendarSqliteProvider::new(
                    manifest.prefix,
                    db_path,
                )));
            }
            other => eprintln!(
                "[plugins] Unknown plugin_type '{}' in {:?} -- skipping",
                other, path
            ),
        }
    }
    providers
}
