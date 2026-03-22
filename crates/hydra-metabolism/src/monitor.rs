//! Metabolism monitor — the main tick-based stability enforcement loop.

use crate::constants::{GAMMA_HAT_FLOOR, MAX_INTERVENTIONS_PER_HOUR};
use crate::errors::MetabolismError;
use crate::intervention::{InterventionEvent, InterventionLevel};
use crate::lyapunov::{classify, LyapunovTracker, StabilityClass};
use chrono::{Duration, Utc};

/// The metabolism monitor. Call `tick()` on every ambient loop iteration.
#[derive(Debug)]
pub struct MetabolismMonitor {
    /// Lyapunov value tracker.
    tracker: LyapunovTracker,
    /// History of intervention events.
    interventions: Vec<InterventionEvent>,
    /// The current intervention level.
    current_level: InterventionLevel,
    /// Total ticks processed.
    tick_count: u64,
}

impl MetabolismMonitor {
    /// Create a new monitor.
    pub fn new() -> Self {
        Self {
            tracker: LyapunovTracker::new(),
            interventions: Vec::new(),
            current_level: InterventionLevel::None,
            tick_count: 0,
        }
    }

    /// Process one tick with the current Lyapunov value and gamma-hat.
    ///
    /// Growth invariant is checked FIRST: if gamma-hat < floor, an error
    /// is returned immediately. Lyapunov violations are NEVER silently ignored.
    pub fn tick(
        &mut self,
        lyapunov_value: f64,
        gamma_hat: f64,
    ) -> Result<InterventionLevel, MetabolismError> {
        // Growth invariant check FIRST
        if gamma_hat < GAMMA_HAT_FLOOR {
            return Err(MetabolismError::GrowthInvariantViolation {
                gamma_hat,
                floor: GAMMA_HAT_FLOOR,
            });
        }

        // Validate the Lyapunov value
        if !lyapunov_value.is_finite() {
            return Err(MetabolismError::NonFiniteLyapunov {
                value: lyapunov_value,
            });
        }

        self.tick_count += 1;
        self.tracker.record(lyapunov_value);

        let class = classify(lyapunov_value);
        let level = self.determine_level(class);

        // If intervention is needed, record it
        if level > InterventionLevel::None {
            self.check_rate_limit()?;
            let actions = self.build_actions(level);
            let event = InterventionEvent::new(level, lyapunov_value, actions);
            tracing::warn!(
                level = %event.level,
                lyapunov = lyapunov_value,
                "metabolism intervention triggered"
            );
            self.interventions.push(event);
        }

        // Recovery: if we were in intervention and now stable, clear it
        if level == InterventionLevel::None && self.current_level > InterventionLevel::None {
            tracing::info!(
                previous_level = %self.current_level,
                "metabolism recovered to stable"
            );
        }

        self.current_level = level;
        Ok(level)
    }

    /// Return the current intervention level.
    pub fn current_level(&self) -> InterventionLevel {
        self.current_level
    }

    /// Return a reference to the Lyapunov tracker.
    pub fn tracker(&self) -> &LyapunovTracker {
        &self.tracker
    }

    /// Return all intervention events.
    pub fn interventions(&self) -> &[InterventionEvent] {
        &self.interventions
    }

    /// Return the total tick count.
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Determine the intervention level from the stability class.
    fn determine_level(&self, class: StabilityClass) -> InterventionLevel {
        match class {
            StabilityClass::Optimal | StabilityClass::Stable => InterventionLevel::None,
            StabilityClass::Alert => InterventionLevel::Level1Alert,
            StabilityClass::Critical => InterventionLevel::Level2Critical,
            StabilityClass::Emergency => InterventionLevel::Level3Emergency,
        }
    }

    /// Build the list of actions for a given intervention level.
    fn build_actions(&self, level: InterventionLevel) -> Vec<String> {
        match level {
            InterventionLevel::None => vec![],
            InterventionLevel::Level1Alert => {
                vec!["notify_principal".to_string(), "log_alert".to_string()]
            }
            InterventionLevel::Level2Critical => vec![
                "notify_principal".to_string(),
                "reduce_task_load".to_string(),
                "checkpoint_state".to_string(),
            ],
            InterventionLevel::Level3Emergency => vec![
                "notify_principal".to_string(),
                "halt_new_tasks".to_string(),
                "checkpoint_state".to_string(),
                "initiate_safe_shutdown".to_string(),
            ],
        }
    }

    /// Check that we haven't exceeded the intervention rate limit.
    fn check_rate_limit(&self) -> Result<(), MetabolismError> {
        let one_hour_ago = Utc::now() - Duration::hours(1);
        let recent_count = self
            .interventions
            .iter()
            .filter(|e| e.triggered_at > one_hour_ago)
            .count();

        if recent_count >= MAX_INTERVENTIONS_PER_HOUR {
            return Err(MetabolismError::InterventionRateLimited {
                count: recent_count,
                max: MAX_INTERVENTIONS_PER_HOUR,
            });
        }
        Ok(())
    }
}

impl Default for MetabolismMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_tick_no_intervention() {
        let mut m = MetabolismMonitor::new();
        let level = m.tick(0.5, 0.01).expect("tick");
        assert_eq!(level, InterventionLevel::None);
    }

    #[test]
    fn alert_triggers_level1() {
        let mut m = MetabolismMonitor::new();
        let level = m.tick(-0.1, 0.01).expect("tick");
        assert_eq!(level, InterventionLevel::Level1Alert);
        assert_eq!(m.interventions().len(), 1);
    }

    #[test]
    fn critical_triggers_level2() {
        let mut m = MetabolismMonitor::new();
        let level = m.tick(-0.6, 0.01).expect("tick");
        assert_eq!(level, InterventionLevel::Level2Critical);
    }

    #[test]
    fn emergency_triggers_level3() {
        let mut m = MetabolismMonitor::new();
        let level = m.tick(-1.5, 0.01).expect("tick");
        assert_eq!(level, InterventionLevel::Level3Emergency);
    }

    #[test]
    fn growth_invariant_violation() {
        let mut m = MetabolismMonitor::new();
        let result = m.tick(0.5, -0.1);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            MetabolismError::GrowthInvariantViolation { .. }
        ));
    }

    #[test]
    fn recovery_clears_level() {
        let mut m = MetabolismMonitor::new();
        m.tick(-0.6, 0.01).expect("tick");
        assert_eq!(m.current_level(), InterventionLevel::Level2Critical);
        m.tick(0.5, 0.01).expect("tick");
        assert_eq!(m.current_level(), InterventionLevel::None);
    }
}
