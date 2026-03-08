//! Resource monitor — polls system memory, CPU, and disk usage.

use std::time::Instant;

use serde::{Deserialize, Serialize};

/// A snapshot of system resource usage at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    /// Memory usage as a percentage (0.0 to 100.0)
    pub memory_percent: f64,
    /// CPU usage as a percentage (0.0 to 100.0)
    pub cpu_percent: f64,
    /// Available disk space in megabytes
    pub disk_available_mb: u64,
    /// Timestamp of the snapshot
    #[serde(skip)]
    pub taken_at: Option<Instant>,
}

impl Default for ResourceSnapshot {
    fn default() -> Self {
        Self {
            memory_percent: 0.0,
            cpu_percent: 0.0,
            disk_available_mb: u64::MAX,
            taken_at: Some(Instant::now()),
        }
    }
}

/// Monitors system resources via platform APIs
pub struct ResourceMonitor {
    last_snapshot: parking_lot::Mutex<ResourceSnapshot>,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            last_snapshot: parking_lot::Mutex::new(ResourceSnapshot::default()),
        }
    }

    /// Take a fresh resource snapshot from the OS
    pub fn snapshot(&self) -> ResourceSnapshot {
        let snap = ResourceSnapshot {
            memory_percent: Self::read_memory_percent(),
            cpu_percent: Self::read_cpu_percent(),
            disk_available_mb: Self::read_disk_available_mb(),
            taken_at: Some(Instant::now()),
        };
        *self.last_snapshot.lock() = snap.clone();
        snap
    }

    /// Return the last snapshot without re-polling
    pub fn last(&self) -> ResourceSnapshot {
        self.last_snapshot.lock().clone()
    }

    /// Create a monitor with a pre-set snapshot (for testing)
    pub fn with_snapshot(snap: ResourceSnapshot) -> Self {
        Self {
            last_snapshot: parking_lot::Mutex::new(snap),
        }
    }

    // ── Platform-specific resource reading ──

    #[cfg(target_os = "macos")]
    fn read_memory_percent() -> f64 {
        // Use sysctl on macOS
        use std::process::Command;
        let output = Command::new("vm_stat").output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            return Self::parse_vm_stat(&text);
        }
        0.0
    }

    #[cfg(target_os = "linux")]
    fn read_memory_percent() -> f64 {
        if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
            let mut total = 0u64;
            let mut available = 0u64;
            for line in contents.lines() {
                if line.starts_with("MemTotal:") {
                    total = Self::parse_meminfo_kb(line);
                } else if line.starts_with("MemAvailable:") {
                    available = Self::parse_meminfo_kb(line);
                }
            }
            if total > 0 {
                return ((total - available) as f64 / total as f64) * 100.0;
            }
        }
        0.0
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn read_memory_percent() -> f64 {
        0.0
    }

    fn read_cpu_percent() -> f64 {
        // CPU usage requires sampling over time; return 0.0 for instant snapshot
        // In production, this would use a rolling average from a background thread
        0.0
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn read_disk_available_mb() -> u64 {
        use std::process::Command;
        // df -m / — get available space on root filesystem
        let output = Command::new("df").args(["-m", "/"]).output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            // Skip header line, parse available column (4th field)
            if let Some(line) = text.lines().nth(1) {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 4 {
                    return fields[3].parse().unwrap_or(u64::MAX);
                }
            }
        }
        u64::MAX
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn read_disk_available_mb() -> u64 {
        u64::MAX
    }

    #[cfg(target_os = "macos")]
    fn parse_vm_stat(text: &str) -> f64 {
        let page_size: u64 = 16384; // ARM64 macOS default
        let mut free = 0u64;
        let mut active = 0u64;
        let mut inactive = 0u64;
        let mut speculative = 0u64;
        let mut wired = 0u64;

        for line in text.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() != 2 {
                continue;
            }
            let val: u64 = parts[1].trim().trim_end_matches('.').parse().unwrap_or(0);
            match parts[0].trim() {
                "Pages free" => free = val,
                "Pages active" => active = val,
                "Pages inactive" => inactive = val,
                "Pages speculative" => speculative = val,
                "Pages wired down" => wired = val,
                _ => {}
            }
        }

        let total = free + active + inactive + speculative + wired;
        if total == 0 {
            return 0.0;
        }
        let used = active + wired;
        let _ = page_size; // page_size used conceptually; ratios cancel out
        (used as f64 / total as f64) * 100.0
    }

    #[cfg(target_os = "linux")]
    fn parse_meminfo_kb(line: &str) -> u64 {
        line.split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_snapshot() {
        let snap = ResourceSnapshot::default();
        assert_eq!(snap.memory_percent, 0.0);
        assert_eq!(snap.cpu_percent, 0.0);
        assert_eq!(snap.disk_available_mb, u64::MAX);
    }

    #[test]
    fn test_monitor_snapshot() {
        let monitor = ResourceMonitor::new();
        let snap = monitor.snapshot();
        // Memory percent should be a valid range
        assert!(snap.memory_percent >= 0.0 && snap.memory_percent <= 100.0);
        assert!(snap.disk_available_mb > 0);
    }

    #[test]
    fn test_monitor_with_preset() {
        let preset = ResourceSnapshot {
            memory_percent: 75.0,
            cpu_percent: 50.0,
            disk_available_mb: 200,
            taken_at: Some(Instant::now()),
        };
        let monitor = ResourceMonitor::with_snapshot(preset.clone());
        let last = monitor.last();
        assert_eq!(last.memory_percent, 75.0);
        assert_eq!(last.cpu_percent, 50.0);
        assert_eq!(last.disk_available_mb, 200);
    }
}
