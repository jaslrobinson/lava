use std::time::Duration;

use super::{DataProvider, ProviderData};

pub struct ResourceMonitorProvider {
    prev_cpu: Option<(u64, u64)>, // (idle, total) from last read
}

impl ResourceMonitorProvider {
    pub fn new() -> Self {
        Self { prev_cpu: None }
    }

    fn read_cpu(&mut self) -> Option<f64> {
        let stat = std::fs::read_to_string("/proc/stat").ok()?;
        let first_line = stat.lines().next()?;
        let values: Vec<u64> = first_line
            .split_whitespace()
            .skip(1) // skip "cpu"
            .filter_map(|s| s.parse().ok())
            .collect();

        if values.len() < 4 {
            return None;
        }

        let idle = values[3];
        let total: u64 = values.iter().sum();

        let result = if let Some((prev_idle, prev_total)) = self.prev_cpu {
            let idle_delta = idle.saturating_sub(prev_idle);
            let total_delta = total.saturating_sub(prev_total);
            if total_delta > 0 {
                Some(100.0 * (1.0 - idle_delta as f64 / total_delta as f64))
            } else {
                Some(0.0)
            }
        } else {
            None
        };

        self.prev_cpu = Some((idle, total));
        result
    }
}

impl DataProvider for ResourceMonitorProvider {
    fn prefix(&self) -> &str {
        "rm"
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();

        // CPU usage
        if let Some(cpu) = self.read_cpu() {
            data.insert("cpuuse".into(), format!("{:.0}", cpu));
        }

        // Memory from /proc/meminfo
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            let mut mem_total = 0u64;
            let mut mem_available = 0u64;
            let mut swap_total = 0u64;
            let mut swap_free = 0u64;

            for line in meminfo.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let val: u64 = parts[1].parse().unwrap_or(0);
                    match parts[0] {
                        "MemTotal:" => mem_total = val,
                        "MemAvailable:" => mem_available = val,
                        "SwapTotal:" => swap_total = val,
                        "SwapFree:" => swap_free = val,
                        _ => {}
                    }
                }
            }

            data.insert("memtot".into(), format!("{}", mem_total / 1024)); // MB
            data.insert("memfree".into(), format!("{}", mem_available / 1024));
            data.insert(
                "memuse".into(),
                format!("{}", mem_total.saturating_sub(mem_available) / 1024),
            );
            data.insert("swptot".into(), format!("{}", swap_total / 1024));
            data.insert("swpfree".into(), format!("{}", swap_free / 1024));
        }

        // Disk usage for root partition via nix statvfs
        #[cfg(target_os = "linux")]
        {
            use nix::sys::statvfs::statvfs;
            if let Ok(stat) = statvfs("/") {
                let block_size = stat.block_size() as u64;
                let total = stat.blocks() * block_size / (1024 * 1024);
                let free = stat.blocks_available() * block_size / (1024 * 1024);
                data.insert("sdtot".into(), format!("{}", total));
                data.insert("sdfree".into(), format!("{}", free));
            }
        }

        data
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(2)
    }
}
