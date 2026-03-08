//! ConnectivityMonitor — detects online/offline state via periodic ping checks.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Connectivity state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectivityState {
    Online,
    Offline,
    Unknown,
}

/// Configuration for the connectivity monitor
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Endpoint to ping for connectivity check
    pub ping_endpoint: String,
    /// How often to check (seconds)
    pub check_interval: Duration,
    /// Timeout for each ping attempt
    pub ping_timeout: Duration,
    /// Number of consecutive failures before declaring offline
    pub failure_threshold: u32,
    /// Number of consecutive successes before declaring online
    pub success_threshold: u32,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            ping_endpoint: "https://api.anthropic.com".to_string(),
            check_interval: Duration::from_secs(30),
            ping_timeout: Duration::from_secs(5),
            failure_threshold: 3,
            success_threshold: 1,
        }
    }
}

/// Monitors network connectivity with configurable ping checks
pub struct ConnectivityMonitor {
    config: MonitorConfig,
    state: parking_lot::Mutex<ConnectivityState>,
    consecutive_failures: parking_lot::Mutex<u32>,
    consecutive_successes: parking_lot::Mutex<u32>,
    last_check: parking_lot::Mutex<Option<Instant>>,
    last_transition: parking_lot::Mutex<Option<Instant>>,
}

impl ConnectivityMonitor {
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            state: parking_lot::Mutex::new(ConnectivityState::Unknown),
            consecutive_failures: parking_lot::Mutex::new(0),
            consecutive_successes: parking_lot::Mutex::new(0),
            last_check: parking_lot::Mutex::new(None),
            last_transition: parking_lot::Mutex::new(None),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(MonitorConfig::default())
    }

    /// Get the current connectivity state
    pub fn state(&self) -> ConnectivityState {
        *self.state.lock()
    }

    /// Whether we are online
    pub fn is_online(&self) -> bool {
        *self.state.lock() == ConnectivityState::Online
    }

    /// Whether we are offline
    pub fn is_offline(&self) -> bool {
        *self.state.lock() == ConnectivityState::Offline
    }

    /// Duration since last state transition
    pub fn time_in_state(&self) -> Option<Duration> {
        self.last_transition.lock().map(|t| t.elapsed())
    }

    /// Duration since last check
    pub fn time_since_check(&self) -> Option<Duration> {
        self.last_check.lock().map(|t| t.elapsed())
    }

    /// Whether a check is due (based on interval)
    pub fn check_due(&self) -> bool {
        match *self.last_check.lock() {
            None => true,
            Some(last) => last.elapsed() >= self.config.check_interval,
        }
    }

    /// Perform a connectivity check. Returns true if state changed.
    pub async fn check(&self) -> bool {
        let reachable = self.ping().await;
        *self.last_check.lock() = Some(Instant::now());
        self.update_state(reachable)
    }

    /// Manually report a connectivity result (for testing or when a request fails/succeeds).
    /// Returns true if state changed.
    pub fn report(&self, reachable: bool) -> bool {
        *self.last_check.lock() = Some(Instant::now());
        self.update_state(reachable)
    }

    /// Force set state (for testing)
    pub fn force_state(&self, state: ConnectivityState) {
        let old = *self.state.lock();
        if old != state {
            *self.last_transition.lock() = Some(Instant::now());
        }
        *self.state.lock() = state;
        match state {
            ConnectivityState::Online => {
                *self.consecutive_failures.lock() = 0;
                *self.consecutive_successes.lock() = self.config.success_threshold;
            }
            ConnectivityState::Offline => {
                *self.consecutive_successes.lock() = 0;
                *self.consecutive_failures.lock() = self.config.failure_threshold;
            }
            ConnectivityState::Unknown => {
                *self.consecutive_failures.lock() = 0;
                *self.consecutive_successes.lock() = 0;
            }
        }
    }

    /// Get the ping endpoint
    pub fn ping_endpoint(&self) -> &str {
        &self.config.ping_endpoint
    }

    /// Ping the configured endpoint
    async fn ping(&self) -> bool {
        // Use a lightweight HEAD request with short timeout
        let client = reqwest::Client::builder()
            .timeout(self.config.ping_timeout)
            .build();

        let client = match client {
            Ok(c) => c,
            Err(_) => return false,
        };

        client.head(&self.config.ping_endpoint).send().await.is_ok()
    }

    /// Update internal state based on reachability. Returns true if state changed.
    fn update_state(&self, reachable: bool) -> bool {
        let old_state = *self.state.lock();

        if reachable {
            *self.consecutive_failures.lock() = 0;
            let mut successes = self.consecutive_successes.lock();
            *successes = successes.saturating_add(1);

            if *successes >= self.config.success_threshold {
                let new_state = ConnectivityState::Online;
                if old_state != new_state {
                    *self.state.lock() = new_state;
                    *self.last_transition.lock() = Some(Instant::now());
                    return true;
                }
            }
        } else {
            *self.consecutive_successes.lock() = 0;
            let mut failures = self.consecutive_failures.lock();
            *failures = failures.saturating_add(1);

            if *failures >= self.config.failure_threshold {
                let new_state = ConnectivityState::Offline;
                if old_state != new_state {
                    *self.state.lock() = new_state;
                    *self.last_transition.lock() = Some(Instant::now());
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_unknown() {
        let monitor = ConnectivityMonitor::with_defaults();
        assert_eq!(monitor.state(), ConnectivityState::Unknown);
        assert!(!monitor.is_online());
        assert!(!monitor.is_offline());
    }

    #[test]
    fn test_report_online_after_threshold() {
        let monitor = ConnectivityMonitor::with_defaults();
        // Default success_threshold = 1, so one success = online
        let changed = monitor.report(true);
        assert!(changed);
        assert!(monitor.is_online());
    }

    #[test]
    fn test_report_offline_after_threshold() {
        let config = MonitorConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let monitor = ConnectivityMonitor::new(config);
        // First failure — not yet offline
        assert!(!monitor.report(false));
        assert_eq!(monitor.state(), ConnectivityState::Unknown);
        // Second failure — now offline
        assert!(monitor.report(false));
        assert!(monitor.is_offline());
    }

    #[test]
    fn test_transition_online_to_offline() {
        let config = MonitorConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let monitor = ConnectivityMonitor::new(config);
        monitor.report(true);
        assert!(monitor.is_online());
        let changed = monitor.report(false);
        assert!(changed);
        assert!(monitor.is_offline());
    }

    #[test]
    fn test_force_state() {
        let monitor = ConnectivityMonitor::with_defaults();
        monitor.force_state(ConnectivityState::Offline);
        assert!(monitor.is_offline());
        monitor.force_state(ConnectivityState::Online);
        assert!(monitor.is_online());
    }

    #[test]
    fn test_check_due() {
        let config = MonitorConfig {
            check_interval: Duration::from_millis(0),
            ..Default::default()
        };
        let monitor = ConnectivityMonitor::new(config);
        assert!(monitor.check_due());
        monitor.report(true);
        // With 0ms interval, should be due again immediately
        assert!(monitor.check_due());
    }

    #[test]
    fn test_no_state_change_same_state() {
        let monitor = ConnectivityMonitor::with_defaults();
        monitor.report(true);
        // Already online, reporting success again should not change
        let changed = monitor.report(true);
        assert!(!changed);
    }
}
