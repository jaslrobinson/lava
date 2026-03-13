use crate::providers::{DataProvider, ProviderData};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use std::path::PathBuf;
use std::time::Duration;

pub struct CalendarIcsProvider {
    prefix: String,
    glob_pattern: String,
}

impl CalendarIcsProvider {
    pub fn new(prefix: String, glob_pattern: String) -> Self {
        Self { prefix, glob_pattern }
    }
}

struct CalEvent {
    title: String,
    start: DateTime<Local>,
    location: String,
}

impl DataProvider for CalendarIcsProvider {
    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(60)
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();
        let files = find_ics_files(&self.glob_pattern);
        let mut events: Vec<CalEvent> = Vec::new();

        for path in files {
            if let Ok(content) = std::fs::read_to_string(&path) {
                events.extend(parse_ics(&content));
            }
        }

        let now = Local::now();
        let today = now.date_naive();
        let cutoff = now + chrono::Duration::days(30);

        // Filter to upcoming events within 30 days, sort ascending
        let mut upcoming: Vec<CalEvent> = events
            .into_iter()
            .filter(|e| e.start >= now && e.start <= cutoff)
            .collect();
        upcoming.sort_by_key(|e| e.start);

        let today_count = upcoming
            .iter()
            .filter(|e| e.start.date_naive() == today)
            .count();

        data.insert("today_count".into(), today_count.to_string());
        data.insert("event_count".into(), upcoming.len().min(5).to_string());

        if let Some(first) = upcoming.first() {
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

        for (i, event) in upcoming.iter().take(5).enumerate() {
            data.insert(format!("event{i}_title"), event.title.clone());
            data.insert(format!("event{i}_time"), event.start.format("%H:%M").to_string());
            data.insert(format!("event{i}_date"), event.start.format("%a %b %-d").to_string());
            data.insert(format!("event{i}_location"), event.location.clone());
        }

        data
    }
}

/// Expand glob pattern and return matching .ics file paths.
/// Supports `~` expansion and `**/` for recursive directory matching.
fn find_ics_files(pattern: &str) -> Vec<PathBuf> {
    let expanded = expand_tilde(pattern);
    let mut results = Vec::new();

    if let Some(recurse_pos) = expanded.find("**/") {
        // Split into base dir and suffix pattern
        let base = PathBuf::from(&expanded[..recurse_pos]);
        let suffix = &expanded[recurse_pos + 3..];
        // suffix may still contain a single `*` wildcard at the dir level
        let filename_pat: Option<&str> = if suffix.contains('/') {
            None // too complex -- skip
        } else {
            Some(suffix)
        };
        if let Some(pat) = filename_pat {
            walk_dir_recursive(&base, pat, &mut results);
        }
    } else if expanded.contains('*') {
        // Simple single-level wildcard: e.g. ~/.thunderbird/*/calendar-data/*.ics
        expand_simple_glob(&expanded, &mut results);
    } else {
        let p = PathBuf::from(&expanded);
        if p.exists() {
            results.push(p);
        }
    }

    results
}

fn walk_dir_recursive(dir: &PathBuf, filename_pat: &str, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir_recursive(&path, filename_pat, out);
        } else if matches_pattern(
            path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
            filename_pat,
        ) {
            out.push(path);
        }
    }
}

/// Expand a path with at most one `*` wildcard segment at each directory level.
fn expand_simple_glob(pattern: &str, out: &mut Vec<PathBuf>) {
    let parts: Vec<&str> = pattern.splitn(2, '*').collect();
    if parts.len() < 2 {
        let p = PathBuf::from(pattern);
        if p.exists() { out.push(p); }
        return;
    }
    let base = PathBuf::from(parts[0].trim_end_matches('/'));
    let rest = parts[1].trim_start_matches('/');

    let entries = match std::fs::read_dir(&base) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let child = entry.path();
        if rest.contains('*') {
            // More wildcards remain -- recurse with expanded prefix
            let next = format!("{}/{}", child.display(), rest);
            expand_simple_glob(&next, out);
        } else {
            let candidate = child.join(rest);
            if rest.is_empty() {
                if child.is_file() { out.push(child); }
            } else if candidate.exists() {
                out.push(candidate);
            }
        }
    }
}

fn matches_pattern(name: &str, pattern: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        name.starts_with(prefix)
    } else if let Some(suffix) = pattern.strip_prefix('*') {
        name.ends_with(suffix)
    } else if pattern == "*" {
        true
    } else {
        name == pattern
    }
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}/{}", home.display(), &path[2..]);
        }
    }
    path.to_string()
}

/// Parse an ICS file content and return calendar events.
fn parse_ics(content: &str) -> Vec<CalEvent> {
    // Unfold continuation lines (lines starting with space/tab)
    let unfolded = unfold_ics(content);
    let mut events = Vec::new();
    let mut in_event = false;
    let mut title = String::new();
    let mut dtstart_raw = String::new();
    let mut location = String::new();
    let mut cancelled = false;

    for line in unfolded.lines() {
        match line {
            "BEGIN:VEVENT" => {
                in_event = true;
                title.clear();
                dtstart_raw.clear();
                location.clear();
                cancelled = false;
            }
            "END:VEVENT" => {
                if in_event && !cancelled && !dtstart_raw.is_empty() {
                    if let Some(start) = parse_dtstart(&dtstart_raw) {
                        events.push(CalEvent { title: title.clone(), start, location: location.clone() });
                    }
                }
                in_event = false;
            }
            _ if in_event => {
                // Property name may have params: NAME;PARAM=VAL:value
                if let Some(pos) = line.find(':') {
                    let prop_full = &line[..pos];
                    let value = &line[pos + 1..];
                    // Strip params from property name
                    let prop = prop_full.split(';').next().unwrap_or(prop_full);
                    match prop {
                        "SUMMARY" => title = unescape_ics(value),
                        "DTSTART" => dtstart_raw = line.to_string(), // keep full line for param parsing
                        "LOCATION" => location = unescape_ics(value),
                        "STATUS" if value == "CANCELLED" => cancelled = true,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    events
}

fn unfold_ics(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    for line in content.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            result.push_str(line.trim_start());
        } else {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
        }
    }
    result
}

fn unescape_ics(s: &str) -> String {
    s.replace("\\n", "\n")
     .replace("\\N", "\n")
     .replace("\\,", ",")
     .replace("\\;", ";")
     .replace("\\\\", "\\")
}

/// Parse DTSTART line (full line preserved to access params).
/// Returns local DateTime or None if unparseable.
fn parse_dtstart(line: &str) -> Option<DateTime<Local>> {
    // line format: DTSTART[;PARAM=VAL...]:VALUE
    let colon = line.find(':')?;
    let value = &line[colon + 1..];

    // All-day: VALUE=DATE or 8-digit value
    if value.len() == 8 || line[..colon].contains("VALUE=DATE") {
        let date = NaiveDate::parse_from_str(value, "%Y%m%d").ok()?;
        let dt = date.and_hms_opt(0, 0, 0)?;
        return Some(Local.from_local_datetime(&dt).single()?);
    }

    // UTC datetime: ends with Z
    if value.ends_with('Z') {
        let naive = NaiveDateTime::parse_from_str(&value[..value.len()-1], "%Y%m%dT%H%M%S").ok()?;
        let utc: DateTime<chrono::Utc> = chrono::Utc.from_utc_datetime(&naive);
        return Some(utc.with_timezone(&Local));
    }

    // Local/TZID datetime: treat as local (chrono-tz not available)
    let naive = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").ok()?;
    Local.from_local_datetime(&naive).single()
}
