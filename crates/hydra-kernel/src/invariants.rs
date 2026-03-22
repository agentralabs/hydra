//! Constitutional invariant checking at runtime.
//!
//! These invariants run on every ambient tick.
//! If any invariant fails, the kernel enters degraded mode.

use crate::{
    constants::{LYAPUNOV_ALERT_THRESHOLD, LYAPUNOV_CRITICAL_THRESHOLD},
    errors::KernelError,
    state::HydraState,
};
use serde::{Deserialize, Serialize};

/// The result of a single invariant check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantResult {
    /// Name of the invariant.
    pub name: String,
    /// Whether the invariant passed.
    pub passed: bool,
    /// Human-readable message.
    pub message: String,
}

/// The results of all invariant checks for one tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantCheckResults {
    /// All individual results.
    pub results: Vec<InvariantResult>,
    /// Whether all invariants passed.
    pub all_passed: bool,
    /// The step at which these checks ran.
    pub step: u64,
}

impl InvariantCheckResults {
    /// Returns the first failure, if any.
    pub fn first_failure(&self) -> Option<&InvariantResult> {
        self.results.iter().find(|r| !r.passed)
    }
}

/// Run all invariant checks against the current state.
pub fn check_all(state: &HydraState) -> InvariantCheckResults {
    let results = vec![
        check_constitution_reachability(state),
        check_animus_bus_health(state),
        check_lyapunov_stability(state),
        check_growth_invariant(state),
        check_signal_queue_health(state),
        check_trust_field_health(state),
    ];

    let all_passed = results.iter().all(|r| r.passed);

    InvariantCheckResults {
        results,
        all_passed,
        step: state.step_count,
    }
}

/// Invariant 1: The constitution must always be reachable.
/// In production, this would verify the ConstitutionChecker is alive.
/// For now, it always passes (the constitution is compiled in).
pub fn check_constitution_reachability(state: &HydraState) -> InvariantResult {
    // Constitution is always reachable — it is a compiled-in dependency.
    // In the future, this checks that the constitutional identity signal
    // can still be produced (no memory corruption).
    let _ = state;
    InvariantResult {
        name: "constitution-reachability".to_string(),
        passed: true,
        message: "Constitution reachable (compiled-in)".to_string(),
    }
}

/// Invariant 2: The Animus bus must be healthy.
/// Checks that signal routing is not saturated and no orphans are accumulating.
pub fn check_animus_bus_health(state: &HydraState) -> InvariantResult {
    let utilization = state.signal_state.queue_utilization;
    let orphans = state.signal_state.orphans_detected;

    if utilization > 0.95 {
        return InvariantResult {
            name: "animus-bus-health".to_string(),
            passed: false,
            message: format!("Bus utilization critical: {:.1}%", utilization * 100.0),
        };
    }

    if orphans > 100 {
        return InvariantResult {
            name: "animus-bus-health".to_string(),
            passed: false,
            message: format!("Orphan signals accumulating: {orphans}"),
        };
    }

    InvariantResult {
        name: "animus-bus-health".to_string(),
        passed: true,
        message: format!(
            "Bus healthy: {:.1}% utilized, {orphans} orphans",
            utilization * 100.0
        ),
    }
}

/// Invariant 3: Lyapunov stability must hold.
/// V(Psi) must remain above the critical threshold.
pub fn check_lyapunov_stability(state: &HydraState) -> InvariantResult {
    let v = state.lyapunov_value;

    if v < LYAPUNOV_CRITICAL_THRESHOLD {
        return InvariantResult {
            name: "lyapunov-stability".to_string(),
            passed: false,
            message: format!("CRITICAL: V(Psi) = {v:.4} < {LYAPUNOV_CRITICAL_THRESHOLD}"),
        };
    }

    if v < LYAPUNOV_ALERT_THRESHOLD {
        return InvariantResult {
            name: "lyapunov-stability".to_string(),
            passed: true, // alert but not failure
            message: format!("ALERT: V(Psi) = {v:.4} < {LYAPUNOV_ALERT_THRESHOLD}"),
        };
    }

    InvariantResult {
        name: "lyapunov-stability".to_string(),
        passed: true,
        message: format!("Stable: V(Psi) = {v:.4}"),
    }
}

/// Invariant 4: Growth must be non-negative.
/// Hydra never loses capabilities — it only gains them.
pub fn check_growth_invariant(state: &HydraState) -> InvariantResult {
    let rate = state.growth_state.growth_rate;

    if rate < 0.0 {
        return InvariantResult {
            name: "growth-invariant".to_string(),
            passed: false,
            message: format!("Negative growth detected: {rate:.4}"),
        };
    }

    InvariantResult {
        name: "growth-invariant".to_string(),
        passed: true,
        message: format!("Growth rate: {rate:.4}"),
    }
}

/// Invariant 5: Signal queue health.
/// No queue should be completely full for sustained periods.
pub fn check_signal_queue_health(state: &HydraState) -> InvariantResult {
    let dropped = state.signal_state.dropped;

    if dropped > 1000 {
        return InvariantResult {
            name: "signal-queue-health".to_string(),
            passed: false,
            message: format!("Excessive signal drops: {dropped}"),
        };
    }

    InvariantResult {
        name: "signal-queue-health".to_string(),
        passed: true,
        message: format!("Signal drops: {dropped}"),
    }
}

/// Invariant 6: Trust field health.
/// Average trust should not drop below a critical floor.
pub fn check_trust_field_health(state: &HydraState) -> InvariantResult {
    let avg = state.trust_field.average_trust;

    if state.trust_field.adversarial_detected && avg < 0.1 {
        return InvariantResult {
            name: "trust-field-health".to_string(),
            passed: false,
            message: format!("Trust field collapsed under adversarial conditions: avg={avg:.3}"),
        };
    }

    InvariantResult {
        name: "trust-field-health".to_string(),
        passed: true,
        message: format!("Trust field: avg={avg:.3}"),
    }
}

/// Convert invariant check results into a kernel error if any failed.
pub fn results_to_error(results: &InvariantCheckResults) -> Option<KernelError> {
    results
        .first_failure()
        .map(|f| KernelError::InvariantFailed {
            invariant: format!("{}: {}", f.name, f.message),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::HydraState;

    #[test]
    fn initial_state_passes_all_invariants() {
        let state = HydraState::initial();
        let results = check_all(&state);
        assert!(results.all_passed);
        assert_eq!(results.results.len(), 6);
    }

    #[test]
    fn critical_lyapunov_fails() {
        let mut state = HydraState::initial();
        state.lyapunov_value = -1.0;
        let result = check_lyapunov_stability(&state);
        assert!(!result.passed);
    }

    #[test]
    fn bus_saturation_fails() {
        let mut state = HydraState::initial();
        state.signal_state.queue_utilization = 0.99;
        let result = check_animus_bus_health(&state);
        assert!(!result.passed);
    }

    #[test]
    fn negative_growth_fails() {
        let mut state = HydraState::initial();
        state.growth_state.growth_rate = -0.1;
        let result = check_growth_invariant(&state);
        assert!(!result.passed);
    }

    #[test]
    fn excessive_drops_fails() {
        let mut state = HydraState::initial();
        state.signal_state.dropped = 2000;
        let result = check_signal_queue_health(&state);
        assert!(!result.passed);
    }

    #[test]
    fn adversarial_trust_collapse_fails() {
        let mut state = HydraState::initial();
        state.trust_field.adversarial_detected = true;
        state.trust_field.average_trust = 0.05;
        let result = check_trust_field_health(&state);
        assert!(!result.passed);
    }

    #[test]
    fn first_failure_returns_correct_invariant() {
        let mut state = HydraState::initial();
        state.lyapunov_value = -1.0;
        let results = check_all(&state);
        assert!(!results.all_passed);
        let failure = results.first_failure().expect("should have failure");
        assert_eq!(failure.name, "lyapunov-stability");
    }

    #[test]
    fn results_to_error_some_on_failure() {
        let mut state = HydraState::initial();
        state.lyapunov_value = -1.0;
        let results = check_all(&state);
        assert!(results_to_error(&results).is_some());
    }

    #[test]
    fn results_to_error_none_on_success() {
        let state = HydraState::initial();
        let results = check_all(&state);
        assert!(results_to_error(&results).is_none());
    }
}
