//! Kernel health reporting — captures a snapshot for the TUI.

use crate::{
    constants::KERNEL_VERSION,
    invariants,
    state::{HydraState, KernelPhase},
    task_engine::TaskEngine,
};
use serde::{Deserialize, Serialize};

/// A health snapshot of the kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelHealth {
    /// Kernel version.
    pub version: String,
    /// Current phase.
    pub phase: KernelPhase,
    /// Current Lyapunov value.
    pub lyapunov_value: f64,
    /// Step count.
    pub step_count: u64,
    /// Number of active tasks.
    pub active_tasks: usize,
    /// Number of total tasks.
    pub total_tasks: usize,
    /// Whether all invariants pass.
    pub invariants_ok: bool,
    /// Number of invariant failures.
    pub invariant_failures: usize,
    /// Signal queue utilization.
    pub signal_utilization: f64,
    /// Average trust level.
    pub average_trust: f64,
    /// Whether adversarial conditions are detected.
    pub adversarial: bool,
}

impl KernelHealth {
    /// Capture a health snapshot from current state.
    pub fn capture(state: &HydraState, phase: &KernelPhase, engine: &TaskEngine) -> Self {
        let invariant_results = invariants::check_all(state);
        let failures = invariant_results
            .results
            .iter()
            .filter(|r| !r.passed)
            .count();

        Self {
            version: KERNEL_VERSION.to_string(),
            phase: phase.clone(),
            lyapunov_value: state.lyapunov_value,
            step_count: state.step_count,
            active_tasks: engine.active_count(),
            total_tasks: engine.total_count(),
            invariants_ok: invariant_results.all_passed,
            invariant_failures: failures,
            signal_utilization: state.signal_state.queue_utilization,
            average_trust: state.trust_field.average_trust,
            adversarial: state.trust_field.adversarial_detected,
        }
    }

    /// Produce a single-line status for TUI display.
    pub fn status_line(&self) -> String {
        let status = if self.invariants_ok { "OK" } else { "DEGRADED" };
        format!(
            "[hydra-kernel v{}] {} | step={} V(Psi)={:.3} | tasks={}/{} | signal={:.0}% | trust={:.2}{}",
            self.version,
            status,
            self.step_count,
            self.lyapunov_value,
            self.active_tasks,
            self.total_tasks,
            self.signal_utilization * 100.0,
            self.average_trust,
            if self.adversarial { " ADVERSARIAL" } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::HydraState;

    #[test]
    fn capture_initial_health() {
        let state = HydraState::initial();
        let phase = KernelPhase::Alive;
        let engine = TaskEngine::new();
        let health = KernelHealth::capture(&state, &phase, &engine);
        assert!(health.invariants_ok);
        assert_eq!(health.step_count, 0);
        assert_eq!(health.active_tasks, 0);
        assert_eq!(health.version, KERNEL_VERSION);
    }

    #[test]
    fn status_line_contains_version() {
        let state = HydraState::initial();
        let phase = KernelPhase::Alive;
        let engine = TaskEngine::new();
        let health = KernelHealth::capture(&state, &phase, &engine);
        let line = health.status_line();
        assert!(line.contains(KERNEL_VERSION));
        assert!(line.contains("OK"));
    }

    #[test]
    fn degraded_status_line() {
        let mut state = HydraState::initial();
        state.lyapunov_value = -1.0;
        let phase = KernelPhase::Degraded {
            reason: "test".to_string(),
        };
        let engine = TaskEngine::new();
        let health = KernelHealth::capture(&state, &phase, &engine);
        assert!(!health.invariants_ok);
        let line = health.status_line();
        assert!(line.contains("DEGRADED"));
    }

    #[test]
    fn adversarial_flag_shows_in_status() {
        let mut state = HydraState::initial();
        state.trust_field.adversarial_detected = true;
        let phase = KernelPhase::Alive;
        let engine = TaskEngine::new();
        let health = KernelHealth::capture(&state, &phase, &engine);
        assert!(health.adversarial);
        let line = health.status_line();
        assert!(line.contains("ADVERSARIAL"));
    }
}
