//! O16 Poller framework — pull-based monitors for external services.
//! HTTP endpoints, process checks, port checks. Each with timeout and failure tracking.

use std::time::Instant;
use super::events::{EventCategory, MonitorEvent};

/// Source type for a poller.
#[derive(Debug, Clone)]
pub enum PollerSource {
    /// HTTP GET — check status code matches expected.
    HttpEndpoint { url: String, expect_status: u16 },
    /// Process check — verify a named process is running.
    ProcessCheck { name: String, expect_running: bool },
    /// Port check — TCP connect to verify port is open.
    PortCheck { port: u16, expect_open: bool },
}

/// A poll-based monitor that checks an external source at intervals.
pub struct Poller {
    pub name: String,
    pub source: PollerSource,
    pub interval_secs: u64,
    pub last_check: Option<Instant>,
    pub last_state: Option<String>,
    /// EC-16.7: per-monitor timeout in seconds.
    pub timeout_secs: u64,
    pub consecutive_failures: u32,
    pub enabled: bool,
}

impl Poller {
    pub fn new(name: &str, source: PollerSource, interval_secs: u64) -> Self {
        Self {
            name: name.to_string(), source, interval_secs,
            last_check: None, last_state: None,
            timeout_secs: 5, consecutive_failures: 0, enabled: true,
        }
    }

    /// Whether it's time to poll (interval elapsed).
    pub fn should_poll(&self) -> bool {
        if !self.enabled { return false; }
        match self.last_check {
            None => true,
            Some(last) => {
                // EC-16.7: Back off on consecutive failures
                let effective_interval = if self.consecutive_failures > 3 {
                    self.interval_secs * 2
                } else {
                    self.interval_secs
                };
                last.elapsed().as_secs() >= effective_interval
            }
        }
    }

    /// Execute the poll. Returns an event if state changed.
    pub fn poll(&mut self) -> Option<MonitorEvent> {
        if !self.enabled { return None; }
        self.last_check = Some(Instant::now());

        let (new_state, category) = match &self.source {
            PollerSource::HttpEndpoint { url, expect_status } => {
                (poll_http(url, *expect_status, self.timeout_secs), EventCategory::Infrastructure)
            }
            PollerSource::ProcessCheck { name, expect_running } => {
                (poll_process(name, *expect_running), EventCategory::Infrastructure)
            }
            PollerSource::PortCheck { port, expect_open } => {
                (poll_port(*port, *expect_open), EventCategory::Infrastructure)
            }
        };

        let state_str = match &new_state {
            Ok(s) => { self.consecutive_failures = 0; s.clone() }
            Err(e) => {
                self.consecutive_failures += 1;
                eprintln!("hydra-monitor: {} poll failed ({}x): {e}", self.name, self.consecutive_failures);
                format!("error: {e}")
            }
        };

        // Only emit event if state changed
        let changed = self.last_state.as_ref() != Some(&state_str);
        self.last_state = Some(state_str.clone());

        if changed {
            let title = format!("{}: {}", self.name, if new_state.is_ok() { "state changed" } else { "check failed" });
            Some(MonitorEvent::new(&self.name, category, &title, &state_str))
        } else {
            None
        }
    }
}

/// HTTP GET check with timeout. Returns status description.
fn poll_http(url: &str, expect_status: u16, timeout_secs: u64) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build().map_err(|e| format!("{e}"))?;
    let resp = client.get(url).send().map_err(|e| {
        // EC-16.1: detect credential expiry
        let msg = e.to_string();
        if msg.contains("401") || msg.contains("403") {
            format!("credential_expired: {msg}")
        } else { msg }
    })?;
    let status = resp.status().as_u16();
    if status == expect_status {
        Ok(format!("ok: {status}"))
    } else if status == 401 || status == 403 {
        Err(format!("credential_expired: got {status}"))
    } else {
        Err(format!("unexpected status: {status} (expected {expect_status})"))
    }
}

/// Check if a named process is running.
fn poll_process(name: &str, expect_running: bool) -> Result<String, String> {
    let output = std::process::Command::new("pgrep").arg("-x").arg(name)
        .output().map_err(|e| format!("{e}"))?;
    let running = output.status.success();
    if running == expect_running {
        Ok(format!("{name}: {}", if running { "running" } else { "stopped" }))
    } else {
        Err(format!("{name}: expected {} but is {}",
            if expect_running { "running" } else { "stopped" },
            if running { "running" } else { "stopped" }))
    }
}

/// TCP connect check for a port.
fn poll_port(port: u16, expect_open: bool) -> Result<String, String> {
    let addr = format!("127.0.0.1:{port}");
    let open = std::net::TcpStream::connect_timeout(
        &addr.parse().unwrap(), std::time::Duration::from_secs(2)).is_ok();
    if open == expect_open {
        Ok(format!("port {port}: {}", if open { "open" } else { "closed" }))
    } else {
        Err(format!("port {port}: expected {} but is {}",
            if expect_open { "open" } else { "closed" },
            if open { "open" } else { "closed" }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_poll_first_time() {
        let p = Poller::new("test", PollerSource::PortCheck { port: 9999, expect_open: false }, 60);
        assert!(p.should_poll());
    }

    #[test]
    fn disabled_poller_no_poll() {
        let mut p = Poller::new("test", PollerSource::PortCheck { port: 9999, expect_open: false }, 60);
        p.enabled = false;
        assert!(!p.should_poll());
    }

    #[test]
    fn http_poller_creates() {
        let p = Poller::new("api", PollerSource::HttpEndpoint {
            url: "http://localhost:99999".into(), expect_status: 200,
        }, 30);
        assert_eq!(p.name, "api");
        assert_eq!(p.interval_secs, 30);
    }

    #[test]
    fn port_check_closed_port() {
        // Port 59999 should be closed
        let result = poll_port(59999, false);
        assert!(result.is_ok());
    }
}
