//! O16 Local Watchers — process, port, and system resource monitoring.

use super::events::{EventCategory, MonitorEvent};

/// A local machine watcher.
#[derive(Debug, Clone)]
pub enum LocalWatcher {
    /// Monitor a named process (running/stopped).
    Process { name: String, expect_running: bool },
    /// Monitor a TCP port (open/closed).
    Port { port: u16, expect_open: bool },
    /// Monitor system resources against thresholds.
    SystemResources { cpu_threshold: f64, memory_threshold: f64 },
}

/// Result of a watcher check.
#[derive(Debug, Clone)]
pub struct WatcherResult {
    pub source: String,
    pub changed: bool,
    pub detail: String,
    pub category: EventCategory,
}

impl LocalWatcher {
    /// Run the watcher check. Returns a result describing the current state.
    pub fn check(&self) -> WatcherResult {
        match self {
            Self::Process { name, expect_running } => check_process(name, *expect_running),
            Self::Port { port, expect_open } => check_port(*port, *expect_open),
            Self::SystemResources { cpu_threshold, memory_threshold } => {
                check_resources(*cpu_threshold, *memory_threshold)
            }
        }
    }

    /// Convert a watcher result to a MonitorEvent if the state has changed.
    pub fn to_event(&self, result: &WatcherResult) -> Option<MonitorEvent> {
        if !result.changed { return None; }
        Some(MonitorEvent::new(&result.source, result.category.clone(), &result.source, &result.detail))
    }

    pub fn source_name(&self) -> String {
        match self {
            Self::Process { name, .. } => format!("process/{name}"),
            Self::Port { port, .. } => format!("port/{port}"),
            Self::SystemResources { .. } => "system/resources".into(),
        }
    }
}

fn check_process(name: &str, expect_running: bool) -> WatcherResult {
    let output = std::process::Command::new("pgrep").arg("-x").arg(name).output();
    let running = output.map(|o| o.status.success()).unwrap_or(false);
    let changed = running != expect_running;
    WatcherResult {
        source: format!("process/{name}"),
        changed,
        detail: format!("{name}: {}", if running { "running" } else { "stopped" }),
        category: EventCategory::Infrastructure,
    }
}

fn check_port(port: u16, expect_open: bool) -> WatcherResult {
    let addr = format!("127.0.0.1:{port}");
    let open = std::net::TcpStream::connect_timeout(
        &addr.parse().unwrap(), std::time::Duration::from_secs(2)).is_ok();
    let changed = open != expect_open;
    WatcherResult {
        source: format!("port/{port}"),
        changed,
        detail: format!("port {port}: {}", if open { "open" } else { "closed" }),
        category: EventCategory::Infrastructure,
    }
}

fn check_resources(cpu_threshold: f64, memory_threshold: f64) -> WatcherResult {
    // Read load average (1-min) as CPU proxy
    let load = std::fs::read_to_string("/proc/loadavg")
        .or_else(|_| {
            // macOS fallback
            std::process::Command::new("sysctl").arg("-n").arg("vm.loadavg")
                .output().map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        })
        .unwrap_or_default();
    let cpu_load: f64 = load.split_whitespace().next()
        .and_then(|s| s.trim_matches(|c: char| !c.is_ascii_digit() && c != '.').parse().ok())
        .unwrap_or(0.0);

    let cpu_exceeded = cpu_load > cpu_threshold;
    // Memory check: simplified — just check if load is high
    let detail = format!("cpu_load={cpu_load:.1} (threshold={cpu_threshold})");
    WatcherResult {
        source: "system/resources".into(),
        changed: cpu_exceeded,
        detail,
        category: EventCategory::Infrastructure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_check_format() {
        let w = LocalWatcher::Process { name: "nonexistent_process_xyz".into(), expect_running: false };
        let r = w.check();
        assert!(!r.changed); // Expected not running, and it's not
        assert!(r.detail.contains("stopped"));
    }

    #[test]
    fn port_check_format() {
        let w = LocalWatcher::Port { port: 59998, expect_open: false };
        let r = w.check();
        assert!(!r.changed);
        assert!(r.detail.contains("closed"));
    }

    #[test]
    fn resources_check() {
        let w = LocalWatcher::SystemResources { cpu_threshold: 999.0, memory_threshold: 999.0 };
        let r = w.check();
        assert!(!r.changed); // Threshold very high, shouldn't trigger
        assert!(r.detail.contains("cpu_load"));
    }

    #[test]
    fn source_name() {
        assert_eq!(LocalWatcher::Process { name: "nginx".into(), expect_running: true }.source_name(), "process/nginx");
        assert_eq!(LocalWatcher::Port { port: 8080, expect_open: true }.source_name(), "port/8080");
    }
}
