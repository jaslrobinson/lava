use std::time::Duration;

use chrono::Local;

use super::{DataProvider, ProviderData};

pub struct DateTimeProvider;

impl DataProvider for DateTimeProvider {
    fn prefix(&self) -> &str {
        "dt"
    }

    fn poll(&mut self) -> ProviderData {
        let now = Local::now();
        let mut data = ProviderData::new();
        data.insert("epoch".into(), now.timestamp().to_string());
        data.insert("h".into(), now.format("%-I").to_string()); // 12h no pad
        data.insert("hh".into(), now.format("%I").to_string()); // 12h padded
        data.insert("H".into(), now.format("%-H").to_string()); // 24h no pad
        data.insert("HH".into(), now.format("%H").to_string()); // 24h padded
        data.insert("m".into(), now.format("%-M").to_string());
        data.insert("mm".into(), now.format("%M").to_string());
        data.insert("s".into(), now.format("%-S").to_string());
        data.insert("ss".into(), now.format("%S").to_string());
        data.insert("a".into(), now.format("%P").to_string()); // am/pm
        data.insert("A".into(), now.format("%p").to_string()); // AM/PM
        data.insert("d".into(), now.format("%-d").to_string());
        data.insert("dd".into(), now.format("%d").to_string());
        data.insert("M".into(), now.format("%-m").to_string());
        data.insert("MM".into(), now.format("%m").to_string());
        data.insert("EEEE".into(), now.format("%A").to_string());
        data.insert("EEE".into(), now.format("%a").to_string());
        data
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(1)
    }
}
