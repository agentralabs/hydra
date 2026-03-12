//! Resource probe — detects RAM, CPU cores, disk space.

use super::os_probe::run_cmd;

/// System resource information.
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub ram_bytes: u64,
    pub cpu_cores: u32,
    pub disk_free_bytes: u64,
    pub disk_total_bytes: u64,
}

impl ResourceInfo {
    pub fn ram_gb(&self) -> f64 {
        self.ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn disk_free_gb(&self) -> f64 {
        self.disk_free_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn display(&self) -> String {
        format!(
            "{:.0}GB RAM, {} cores, {:.0}GB free disk",
            self.ram_gb(),
            self.cpu_cores,
            self.disk_free_gb()
        )
    }
}

/// Probe system resources.
pub fn probe_resources() -> ResourceInfo {
    ResourceInfo {
        ram_bytes: probe_ram(),
        cpu_cores: probe_cpu_cores(),
        disk_free_bytes: probe_disk_free(),
        disk_total_bytes: probe_disk_total(),
    }
}

fn probe_ram() -> u64 {
    if cfg!(target_os = "macos") {
        // sysctl hw.memsize returns bytes
        run_cmd("sysctl", &["-n", "hw.memsize"])
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0)
    } else {
        // Linux: /proc/meminfo
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|content| {
                content.lines()
                    .find(|l| l.starts_with("MemTotal:"))
                    .and_then(|l| {
                        l.split_whitespace().nth(1)
                            .and_then(|v| v.parse::<u64>().ok())
                            .map(|kb| kb * 1024) // Convert kB to bytes
                    })
            })
            .unwrap_or(0)
    }
}

fn probe_cpu_cores() -> u32 {
    if cfg!(target_os = "macos") {
        run_cmd("sysctl", &["-n", "hw.ncpu"])
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(1)
    } else {
        run_cmd("nproc", &[])
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(1)
    }
}

fn probe_disk_free() -> u64 {
    // df output: filesystem, blocks, used, available, capacity, mount
    // Use 1024-byte blocks for consistent parsing
    let output = if cfg!(target_os = "macos") {
        run_cmd("df", &["-k", "."])
    } else {
        run_cmd("df", &["-k", "."])
    };

    output
        .and_then(|s| {
            s.lines().nth(1).and_then(|line| {
                // Available is typically the 4th column
                line.split_whitespace().nth(3)
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(|kb| kb * 1024) // Convert kB to bytes
            })
        })
        .unwrap_or(0)
}

fn probe_disk_total() -> u64 {
    let output = run_cmd("df", &["-k", "."]);
    output
        .and_then(|s| {
            s.lines().nth(1).and_then(|line| {
                // Total is typically the 2nd column
                line.split_whitespace().nth(1)
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(|kb| kb * 1024)
            })
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_resources() {
        let r = probe_resources();
        assert!(r.ram_bytes > 0, "Should detect some RAM");
        assert!(r.cpu_cores > 0, "Should detect at least 1 CPU core");
        assert!(r.disk_free_bytes > 0, "Should detect free disk space");
    }

    #[test]
    fn test_ram_gb() {
        let r = ResourceInfo {
            ram_bytes: 16 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            disk_free_bytes: 200 * 1024 * 1024 * 1024,
            disk_total_bytes: 500 * 1024 * 1024 * 1024,
        };
        assert!((r.ram_gb() - 16.0).abs() < 0.01);
        assert!((r.disk_free_gb() - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_display() {
        let r = ResourceInfo {
            ram_bytes: 16 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            disk_free_bytes: 200 * 1024 * 1024 * 1024,
            disk_total_bytes: 500 * 1024 * 1024 * 1024,
        };
        let d = r.display();
        assert!(d.contains("16GB RAM"));
        assert!(d.contains("8 cores"));
        assert!(d.contains("200GB free"));
    }
}
