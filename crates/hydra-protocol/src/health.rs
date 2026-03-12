use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Health status of a protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Health record for a protocol
#[derive(Debug, Clone)]
struct HealthRecord {
    status: HealthStatus,
    last_checked: DateTime<Utc>,
    consecutive_failures: u32,
    total_checks: u64,
    total_successes: u64,
}

impl Default for HealthRecord {
    fn default() -> Self {
        Self {
            status: HealthStatus::Unknown,
            last_checked: Utc::now(),
            consecutive_failures: 0,
            total_checks: 0,
            total_successes: 0,
        }
    }
}

/// Tracks health of all registered protocols
pub struct HealthTracker {
    records: DashMap<Uuid, HealthRecord>,
    unhealthy_threshold: u32,
    degraded_threshold: u32,
    total_health_checks: AtomicU64,
}

impl HealthTracker {
    pub fn new() -> Self {
        Self {
            records: DashMap::new(),
            unhealthy_threshold: 3,
            degraded_threshold: 1,
            total_health_checks: AtomicU64::new(0),
        }
    }

    /// Mark a protocol as healthy after a successful operation
    pub fn mark_healthy(&self, protocol_id: Uuid) {
        self.total_health_checks.fetch_add(1, Ordering::Relaxed);
        let mut entry = self.records.entry(protocol_id).or_default();
        entry.status = HealthStatus::Healthy;
        entry.last_checked = Utc::now();
        entry.consecutive_failures = 0;
        entry.total_checks += 1;
        entry.total_successes += 1;
    }

    /// Mark a protocol as unhealthy after a failed operation
    pub fn mark_unhealthy(&self, protocol_id: Uuid) {
        self.total_health_checks.fetch_add(1, Ordering::Relaxed);
        let mut entry = self.records.entry(protocol_id).or_default();
        entry.consecutive_failures += 1;
        entry.total_checks += 1;
        entry.last_checked = Utc::now();

        entry.status = if entry.consecutive_failures >= self.unhealthy_threshold {
            HealthStatus::Unhealthy
        } else if entry.consecutive_failures >= self.degraded_threshold {
            HealthStatus::Degraded
        } else {
            entry.status
        };
    }

    /// Get health status of a protocol
    pub fn check_health(&self, protocol_id: Uuid) -> HealthStatus {
        self.records
            .get(&protocol_id)
            .map(|r| r.status)
            .unwrap_or(HealthStatus::Unknown)
    }

    /// Check if a protocol is healthy enough to use
    pub fn is_available(&self, protocol_id: Uuid) -> bool {
        matches!(
            self.check_health(protocol_id),
            HealthStatus::Healthy | HealthStatus::Unknown
        )
    }

    /// Get uptime ratio for a protocol
    pub fn uptime_ratio(&self, protocol_id: Uuid) -> f64 {
        self.records
            .get(&protocol_id)
            .map(|r| {
                if r.total_checks == 0 {
                    1.0
                } else {
                    r.total_successes as f64 / r.total_checks as f64
                }
            })
            .unwrap_or(1.0)
    }

    /// Mark all protocols as unhealthy
    pub fn mark_all_unhealthy(&self) {
        for mut entry in self.records.iter_mut() {
            entry.status = HealthStatus::Unhealthy;
            entry.consecutive_failures = self.unhealthy_threshold;
        }
    }

    /// Total health checks performed
    pub fn total_checks(&self) -> u64 {
        self.total_health_checks.load(Ordering::Relaxed)
    }

    /// Get all protocol IDs that are currently unhealthy (for recovery pings)
    pub fn unhealthy_protocols(&self) -> Vec<Uuid> {
        self.records
            .iter()
            .filter(|r| r.status == HealthStatus::Unhealthy)
            .map(|r| *r.key())
            .collect()
    }

    /// Start a background auto-recovery task that periodically pings unhealthy protocols.
    /// The `check_fn` is called for each unhealthy protocol. If it returns true, the protocol
    /// is marked healthy again.
    pub fn start_auto_recovery(
        self: &std::sync::Arc<Self>,
        interval: std::time::Duration,
        check_fn: std::sync::Arc<dyn Fn(Uuid) -> bool + Send + Sync>,
    ) -> tokio::task::JoinHandle<()> {
        let tracker = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                let unhealthy = tracker.unhealthy_protocols();
                for id in unhealthy {
                    if check_fn(id) {
                        tracker.mark_healthy(id);
                    }
                }
            }
        })
    }
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialization() {
        assert_eq!(serde_json::to_string(&HealthStatus::Healthy).unwrap(), "\"healthy\"");
        assert_eq!(serde_json::to_string(&HealthStatus::Degraded).unwrap(), "\"degraded\"");
        assert_eq!(serde_json::to_string(&HealthStatus::Unhealthy).unwrap(), "\"unhealthy\"");
        assert_eq!(serde_json::to_string(&HealthStatus::Unknown).unwrap(), "\"unknown\"");
    }

    #[test]
    fn test_health_tracker_new() {
        let tracker = HealthTracker::new();
        assert_eq!(tracker.total_checks(), 0);
    }

    #[test]
    fn test_health_tracker_default() {
        let tracker = HealthTracker::default();
        assert_eq!(tracker.total_checks(), 0);
    }

    #[test]
    fn test_unknown_protocol_health() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        assert_eq!(tracker.check_health(id), HealthStatus::Unknown);
    }

    #[test]
    fn test_mark_healthy() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.mark_healthy(id);
        assert_eq!(tracker.check_health(id), HealthStatus::Healthy);
        assert_eq!(tracker.total_checks(), 1);
    }

    #[test]
    fn test_mark_unhealthy_once_degraded() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.mark_healthy(id); // start healthy
        tracker.mark_unhealthy(id); // 1 failure => degraded (threshold=1)
        assert_eq!(tracker.check_health(id), HealthStatus::Degraded);
    }

    #[test]
    fn test_mark_unhealthy_three_times() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.mark_unhealthy(id);
        tracker.mark_unhealthy(id);
        tracker.mark_unhealthy(id); // 3 >= unhealthy_threshold(3)
        assert_eq!(tracker.check_health(id), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_is_available_healthy() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.mark_healthy(id);
        assert!(tracker.is_available(id));
    }

    #[test]
    fn test_is_available_unknown() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        assert!(tracker.is_available(id)); // Unknown is available
    }

    #[test]
    fn test_is_available_unhealthy() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.mark_unhealthy(id);
        tracker.mark_unhealthy(id);
        tracker.mark_unhealthy(id);
        assert!(!tracker.is_available(id));
    }

    #[test]
    fn test_is_available_degraded() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.mark_unhealthy(id); // 1 failure = degraded
        assert!(!tracker.is_available(id));
    }

    #[test]
    fn test_uptime_ratio_no_checks() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        assert_eq!(tracker.uptime_ratio(id), 1.0); // Default
    }

    #[test]
    fn test_uptime_ratio_all_healthy() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.mark_healthy(id);
        tracker.mark_healthy(id);
        tracker.mark_healthy(id);
        assert_eq!(tracker.uptime_ratio(id), 1.0);
    }

    #[test]
    fn test_uptime_ratio_mixed() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.mark_healthy(id);
        tracker.mark_unhealthy(id);
        // 1 success, 2 total checks => 0.5
        assert_eq!(tracker.uptime_ratio(id), 0.5);
    }

    #[test]
    fn test_mark_all_unhealthy() {
        let tracker = HealthTracker::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        tracker.mark_healthy(id1);
        tracker.mark_healthy(id2);
        tracker.mark_all_unhealthy();
        assert_eq!(tracker.check_health(id1), HealthStatus::Unhealthy);
        assert_eq!(tracker.check_health(id2), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_unhealthy_protocols_list() {
        let tracker = HealthTracker::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        tracker.mark_healthy(id1);
        tracker.mark_unhealthy(id2);
        tracker.mark_unhealthy(id2);
        tracker.mark_unhealthy(id2);
        let unhealthy = tracker.unhealthy_protocols();
        assert_eq!(unhealthy.len(), 1);
        assert!(unhealthy.contains(&id2));
    }

    #[test]
    fn test_healthy_recovery_resets_failures() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.mark_unhealthy(id);
        tracker.mark_unhealthy(id);
        tracker.mark_healthy(id); // Recovery
        assert_eq!(tracker.check_health(id), HealthStatus::Healthy);
    }

    #[test]
    fn test_total_checks_accumulate() {
        let tracker = HealthTracker::new();
        let id = Uuid::new_v4();
        tracker.mark_healthy(id);
        tracker.mark_unhealthy(id);
        tracker.mark_healthy(id);
        assert_eq!(tracker.total_checks(), 3);
    }
}
