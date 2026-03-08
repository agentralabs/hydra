//! Degradation policy — thresholds and hysteresis to prevent flapping.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::manager::DegradationLevel;
use super::monitor::ResourceSnapshot;

/// Configuration for degradation thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Memory % threshold for Reduced level
    pub memory_reduced: f64,
    /// Memory % threshold for Minimal level
    pub memory_minimal: f64,
    /// Memory % threshold for Emergency level
    pub memory_emergency: f64,
    /// Sustained CPU % threshold for Reduced level
    pub cpu_reduced: f64,
    /// Disk MB below which triggers Minimal
    pub disk_minimal_mb: u64,
    /// How long a threshold must be exceeded before upgrading degradation
    pub hysteresis_secs: u64,
    /// How long below threshold before recovering
    pub recovery_secs: u64,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            memory_reduced: 70.0,
            memory_minimal: 85.0,
            memory_emergency: 95.0,
            cpu_reduced: 90.0,
            disk_minimal_mb: 500,
            hysteresis_secs: 30,
            recovery_secs: 30,
        }
    }
}

/// Evaluates resource snapshots against thresholds with hysteresis
pub struct DegradationPolicy {
    config: PolicyConfig,
    /// When each level was first triggered (for hysteresis)
    trigger_start: parking_lot::Mutex<Option<(DegradationLevel, Instant)>>,
    /// When recovery started
    recovery_start: parking_lot::Mutex<Option<Instant>>,
    /// Manual override (bypasses automatic evaluation)
    manual_override: parking_lot::Mutex<Option<DegradationLevel>>,
}

impl DegradationPolicy {
    pub fn new(config: PolicyConfig) -> Self {
        Self {
            config,
            trigger_start: parking_lot::Mutex::new(None),
            recovery_start: parking_lot::Mutex::new(None),
            manual_override: parking_lot::Mutex::new(None),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(PolicyConfig::default())
    }

    /// Set a manual override level (None to clear)
    pub fn set_override(&self, level: Option<DegradationLevel>) {
        *self.manual_override.lock() = level;
    }

    /// Get current manual override
    pub fn get_override(&self) -> Option<DegradationLevel> {
        *self.manual_override.lock()
    }

    /// Get the current policy config
    pub fn config(&self) -> &PolicyConfig {
        &self.config
    }

    /// Evaluate a snapshot and return the recommended level.
    /// Applies hysteresis: level only changes after threshold is sustained.
    pub fn evaluate(
        &self,
        snapshot: &ResourceSnapshot,
        current_level: DegradationLevel,
    ) -> DegradationLevel {
        // Manual override takes precedence
        if let Some(level) = *self.manual_override.lock() {
            return level;
        }

        let recommended = self.raw_evaluate(snapshot);

        if recommended > current_level {
            // Degrading — apply hysteresis before worsening
            let mut trigger = self.trigger_start.lock();
            match &*trigger {
                Some((triggered_level, started_at)) if *triggered_level == recommended => {
                    if started_at.elapsed() >= Duration::from_secs(self.config.hysteresis_secs) {
                        // Sustained long enough — apply
                        *self.recovery_start.lock() = None;
                        return recommended;
                    }
                    // Not sustained long enough
                    return current_level;
                }
                _ => {
                    // New trigger — start the clock
                    *trigger = Some((recommended, Instant::now()));
                    return current_level;
                }
            }
        } else if recommended < current_level {
            // Recovering — apply recovery hysteresis
            let mut recovery = self.recovery_start.lock();
            match &*recovery {
                Some(started_at) => {
                    if started_at.elapsed() >= Duration::from_secs(self.config.recovery_secs) {
                        // Sustained recovery — step down one level
                        *recovery = None;
                        *self.trigger_start.lock() = None;
                        return current_level.step_down();
                    }
                    return current_level;
                }
                None => {
                    *recovery = Some(Instant::now());
                    return current_level;
                }
            }
        } else {
            // Same level — clear timers
            *self.trigger_start.lock() = None;
            *self.recovery_start.lock() = None;
            current_level
        }
    }

    /// Evaluate without hysteresis (raw threshold check)
    pub fn raw_evaluate(&self, snapshot: &ResourceSnapshot) -> DegradationLevel {
        // Check emergency first (most severe)
        if snapshot.memory_percent >= self.config.memory_emergency {
            return DegradationLevel::Emergency;
        }

        // Check minimal conditions
        if snapshot.memory_percent >= self.config.memory_minimal
            || snapshot.disk_available_mb < self.config.disk_minimal_mb
        {
            return DegradationLevel::Minimal;
        }

        // Check reduced conditions
        if snapshot.memory_percent >= self.config.memory_reduced
            || snapshot.cpu_percent >= self.config.cpu_reduced
        {
            return DegradationLevel::Reduced;
        }

        DegradationLevel::Normal
    }

    /// Reset all hysteresis timers (for testing)
    pub fn reset_timers(&self) {
        *self.trigger_start.lock() = None;
        *self.recovery_start.lock() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(memory: f64, cpu: f64, disk: u64) -> ResourceSnapshot {
        ResourceSnapshot {
            memory_percent: memory,
            cpu_percent: cpu,
            disk_available_mb: disk,
            taken_at: Some(Instant::now()),
        }
    }

    #[test]
    fn test_raw_evaluate_normal() {
        let policy = DegradationPolicy::with_defaults();
        assert_eq!(
            policy.raw_evaluate(&snap(30.0, 20.0, 5000)),
            DegradationLevel::Normal
        );
    }

    #[test]
    fn test_raw_evaluate_reduced_memory() {
        let policy = DegradationPolicy::with_defaults();
        assert_eq!(
            policy.raw_evaluate(&snap(75.0, 20.0, 5000)),
            DegradationLevel::Reduced
        );
    }

    #[test]
    fn test_raw_evaluate_reduced_cpu() {
        let policy = DegradationPolicy::with_defaults();
        assert_eq!(
            policy.raw_evaluate(&snap(30.0, 95.0, 5000)),
            DegradationLevel::Reduced
        );
    }

    #[test]
    fn test_raw_evaluate_minimal_memory() {
        let policy = DegradationPolicy::with_defaults();
        assert_eq!(
            policy.raw_evaluate(&snap(88.0, 20.0, 5000)),
            DegradationLevel::Minimal
        );
    }

    #[test]
    fn test_raw_evaluate_minimal_disk() {
        let policy = DegradationPolicy::with_defaults();
        assert_eq!(
            policy.raw_evaluate(&snap(30.0, 20.0, 300)),
            DegradationLevel::Minimal
        );
    }

    #[test]
    fn test_raw_evaluate_emergency() {
        let policy = DegradationPolicy::with_defaults();
        assert_eq!(
            policy.raw_evaluate(&snap(96.0, 50.0, 5000)),
            DegradationLevel::Emergency
        );
    }

    #[test]
    fn test_manual_override() {
        let policy = DegradationPolicy::with_defaults();
        policy.set_override(Some(DegradationLevel::Minimal));
        // Even with normal resources, manual override wins
        let result = policy.evaluate(&snap(10.0, 5.0, 10000), DegradationLevel::Normal);
        assert_eq!(result, DegradationLevel::Minimal);

        policy.set_override(None);
        let result = policy.evaluate(&snap(10.0, 5.0, 10000), DegradationLevel::Normal);
        assert_eq!(result, DegradationLevel::Normal);
    }
}
