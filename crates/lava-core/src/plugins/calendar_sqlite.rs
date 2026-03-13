use crate::providers::{DataProvider, ProviderData};
use chrono::{DateTime, Local};
use std::path::PathBuf;
use std::time::Duration;

pub struct CalendarSqliteProvider {
    prefix: String,
    db_glob: String,
}

impl CalendarSqliteProvider {
    pub fn new(prefix: String, db_glob: String) -> Self {
        Self { prefix, db_glob }
    }
}

struct CalEvent {
    title: String,
    start: DateTime<Local>,
    location: String,
}

impl DataProvider for CalendarSqliteProvider {
    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(60)
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();
        let db_paths = resolve_db_paths(&self.db_glob);

        let now = Local::now();
        let today = now.date_naive();
        let cutoff = now + chrono::Duration::days(30);

        let now_us = now.timestamp_micros();
        let cutoff_us = cutoff.timestamp_micros();

        let mut events: Vec<CalEvent> = Vec::new();

        for db_path in db_paths {
            if let Ok(conn) = rusqlite::Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            ) {
                let query = "SELECT e.id, e.title, e.event_start, e.event_start_tz, \
                             COALESCE((SELECT value FROM cal_properties WHERE item_id = e.id AND key = 'LOCATION' LIMIT 1), '') \
                             FROM cal_events e \
                             WHERE e.title IS NOT NULL AND e.title != '' \
                             AND (e.ical_status IS NULL OR e.ical_status != 'CANCELLED') \
                             AND e.event_start >= ?1 AND e.event_start <= ?2 \
                             ORDER BY e.event_start ASC LIMIT 20";

                if let Ok(mut stmt) = conn.prepare(query) {
                    let rows = stmt.query_map(
                        rusqlite::params![now_us, cutoff_us],
                        |row| {
                            Ok((
                                row.get::<_, String>(1)?,  // title
                                row.get::<_, i64>(2)?,     // event_start (microseconds)
                                row.get::<_, String>(3)?,  // timezone
                                row.get::<_, String>(4)?,  // location
                            ))
                        },
                    );
                    if let Ok(rows) = rows {
                        for row in rows.flatten() {
                            let (title, start_us, _tz, location) = row;
                            if let Some(start) = micros_to_local(start_us) {
                                events.push(CalEvent { title, start, location });
                            }
                        }
                    }
                }
            }
        }

        events.sort_by_key(|e| e.start);

        let today_count = events.iter().filter(|e| e.start.date_naive() == today).count();

        data.insert("today_count".into(), today_count.to_string());
        data.insert("event_count".into(), events.len().min(5).to_string());

        if let Some(first) = events.first() {
            data.insert("next_title".into(), first.title.clone());
            data.insert("next_time".into(), first.start.format("%H:%M").to_string());
            data.insert("next_date".into(), first.start.format("%a %b %-d").to_string());
            data.insert("next_location".into(), first.location.clone());
        } else {
            data.insert("next_title".into(), String::new());
            data.insert("next_time".into(), String::new());
            data.insert("next_date".into(), String::new());
            data.insert("next_location".into(), String::new());
        }

        for (i, event) in events.iter().take(5).enumerate() {
            data.insert(format!("event{i}_title"), event.title.clone());
            data.insert(format!("event{i}_time"), event.start.format("%H:%M").to_string());
            data.insert(format!("event{i}_date"), event.start.format("%a %b %-d").to_string());
            data.insert(format!("event{i}_location"), event.location.clone());
        }

        data
    }
}

/// Convert Thunderbird's PRTime (microseconds since Unix epoch) to local DateTime.
fn micros_to_local(us: i64) -> Option<DateTime<Local>> {
    let secs = us / 1_000_000;
    let nanos = ((us % 1_000_000) * 1000) as u32;
    let utc = chrono::DateTime::from_timestamp(secs, nanos)?;
    Some(utc.with_timezone(&Local))
}

/// Resolve a glob-style db path (supports ~ and single * wildcard).
fn resolve_db_paths(pattern: &str) -> Vec<PathBuf> {
    let expanded = expand_tilde(pattern);
    let mut results = Vec::new();

    if expanded.contains('*') {
        // Split on first * to get base dir and suffix
        let parts: Vec<&str> = expanded.splitn(2, '*').collect();
        let base = PathBuf::from(parts[0].trim_end_matches('/'));
        let suffix = parts[1].trim_start_matches('/');

        let entries = match std::fs::read_dir(&base) {
            Ok(e) => e,
            Err(_) => return results,
        };
        for entry in entries.flatten() {
            let candidate = entry.path().join(suffix);
            if candidate.exists() {
                results.push(candidate);
            }
        }
    } else {
        let p = PathBuf::from(&expanded);
        if p.exists() {
            results.push(p);
        }
    }

    results
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}/{}", home.display(), &path[2..]);
        }
    }
    path.to_string()
}
