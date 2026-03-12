//! Health summary generator — aggregates system state into a single report.

use std::fmt;

#[derive(Debug, Clone)]
pub struct HealthSummary {
    pub connected_sisters: usize,
    pub total_sisters: usize,
    pub uptime_secs: u64,
    pub last_error: Option<String>,
    pub memory_mb: f64,
}

/// Generate a health summary from system state.
pub fn generate(
    connected: usize,
    total: usize,
    uptime: u64,
    last_err: Option<String>,
) -> HealthSummary {
    // Rough memory estimate from current process (fallback to 0)
    let memory_mb = estimate_memory_mb();
    HealthSummary {
        connected_sisters: connected,
        total_sisters: total,
        uptime_secs: uptime,
        last_error: last_err,
        memory_mb,
    }
}

fn estimate_memory_mb() -> f64 {
    // On macOS/Linux, read from /proc or use a rough estimate
    // This is a best-effort heuristic — no external deps
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let kb: f64 = line.split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    return kb / 1024.0;
                }
            }
        }
        0.0
    }
    #[cfg(not(target_os = "linux"))]
    {
        0.0 // macOS doesn't expose /proc; callers can override
    }
}

/// Create a health summary with an explicit memory value.
pub fn generate_with_memory(
    connected: usize,
    total: usize,
    uptime: u64,
    last_err: Option<String>,
    memory_mb: f64,
) -> HealthSummary {
    HealthSummary {
        connected_sisters: connected,
        total_sisters: total,
        uptime_secs: uptime,
        last_error: last_err,
        memory_mb,
    }
}

impl fmt::Display for HealthSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hours = self.uptime_secs / 3600;
        let mins = (self.uptime_secs % 3600) / 60;
        let secs = self.uptime_secs % 60;

        writeln!(f, "Hydra Health Summary")?;
        writeln!(f, "  Sisters: {}/{} connected", self.connected_sisters, self.total_sisters)?;
        writeln!(f, "  Uptime:  {}h {}m {}s", hours, mins, secs)?;
        writeln!(f, "  Memory:  {:.1} MB", self.memory_mb)?;
        match &self.last_error {
            Some(err) => writeln!(f, "  Last error: {}", err),
            None => writeln!(f, "  Last error: none"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_basic() {
        let h = generate(5, 14, 3661, None);
        assert_eq!(h.connected_sisters, 5);
        assert_eq!(h.total_sisters, 14);
        assert_eq!(h.uptime_secs, 3661);
        assert!(h.last_error.is_none());
    }

    #[test]
    fn test_generate_with_error() {
        let h = generate(3, 14, 120, Some("timeout".into()));
        assert_eq!(h.last_error.as_deref(), Some("timeout"));
    }

    #[test]
    fn test_generate_with_memory() {
        let h = generate_with_memory(10, 14, 7200, None, 256.5);
        assert!((h.memory_mb - 256.5).abs() < 0.01);
    }

    #[test]
    fn test_display_no_error() {
        let h = generate_with_memory(5, 14, 3661, None, 128.0);
        let text = format!("{}", h);
        assert!(text.contains("5/14 connected"));
        assert!(text.contains("1h 1m 1s"));
        assert!(text.contains("128.0 MB"));
        assert!(text.contains("Last error: none"));
    }

    #[test]
    fn test_display_with_error() {
        let h = generate_with_memory(0, 14, 0, Some("builder error".into()), 0.0);
        let text = format!("{}", h);
        assert!(text.contains("0/14 connected"));
        assert!(text.contains("Last error: builder error"));
    }
}
