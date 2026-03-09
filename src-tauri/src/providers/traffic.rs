use std::time::{Duration, Instant};

use super::{DataProvider, ProviderData};

pub struct TrafficProvider {
    prev_rx: u64,
    prev_tx: u64,
    prev_time: Instant,
}

impl TrafficProvider {
    pub fn new() -> Self {
        Self {
            prev_rx: 0,
            prev_tx: 0,
            prev_time: Instant::now(),
        }
    }

    fn read_net_dev(&self) -> (u64, u64) {
        let mut total_rx = 0u64;
        let mut total_tx = 0u64;

        if let Ok(content) = std::fs::read_to_string("/proc/net/dev") {
            for line in content.lines().skip(2) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    let iface = parts[0].trim_end_matches(':');
                    if iface == "lo" {
                        continue;
                    } // skip loopback
                    if let (Ok(rx), Ok(tx)) = (parts[1].parse::<u64>(), parts[9].parse::<u64>()) {
                        total_rx += rx;
                        total_tx += tx;
                    }
                }
            }
        }

        (total_rx, total_tx)
    }
}

impl DataProvider for TrafficProvider {
    fn prefix(&self) -> &str {
        "ts"
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();
        let (rx, tx) = self.read_net_dev();
        let now = Instant::now();
        let elapsed = now.duration_since(self.prev_time).as_secs_f64();

        data.insert("trx".into(), rx.to_string());
        data.insert("ttx".into(), tx.to_string());

        if elapsed > 0.0 && self.prev_rx > 0 {
            let speed_rx = (rx.saturating_sub(self.prev_rx) as f64 / elapsed) as u64;
            let speed_tx = (tx.saturating_sub(self.prev_tx) as f64 / elapsed) as u64;
            data.insert("srx".into(), speed_rx.to_string());
            data.insert("stx".into(), speed_tx.to_string());
        }

        self.prev_rx = rx;
        self.prev_tx = tx;
        self.prev_time = now;

        data
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(2)
    }
}
