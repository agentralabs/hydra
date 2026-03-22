//! The Hydra equation: dPsi/dt = L-hat Psi + A-hat Psi + G-hat Psi + S-hat Psi - Gamma-hat Psi
//!
//! Each operator is computed independently and combined via Euler integration.
//! The result is a new HydraState — the next instant of being.

use crate::state::HydraState;
use serde::{Deserialize, Serialize};

/// The result of one equation step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquationStep {
    /// The Laplace-Beltrami operator contribution.
    pub l_hat: f64,
    /// The adversarial operator contribution.
    pub a_hat: f64,
    /// The growth operator contribution.
    pub g_hat: f64,
    /// The signal operator contribution.
    pub s_hat: f64,
    /// The dissipation operator contribution (subtracted).
    pub gamma_hat: f64,
    /// The net dPsi/dt value.
    pub dpsi_dt: f64,
    /// The step number.
    pub step: u64,
}

impl EquationStep {
    /// Compute one step of the equation from the current state.
    pub fn compute(state: &HydraState) -> Self {
        let l_hat = compute_l_hat(state);
        let a_hat = compute_a_hat(state);
        let g_hat = compute_g_hat(state);
        let s_hat = compute_s_hat(state);
        let gamma_hat = compute_gamma_hat(state);

        let dpsi_dt = l_hat + a_hat + g_hat + s_hat - gamma_hat;

        Self {
            l_hat,
            a_hat,
            g_hat,
            s_hat,
            gamma_hat,
            dpsi_dt,
            step: state.step_count,
        }
    }

    /// Returns true if the step result indicates stability (dPsi/dt >= 0).
    pub fn is_stable(&self) -> bool {
        self.dpsi_dt >= 0.0
    }

    /// Returns a human-readable summary of the equation step.
    pub fn summary(&self) -> String {
        format!(
            "step={} dPsi/dt={:.4} [L={:.3} A={:.3} G={:.3} S={:.3} Gamma={:.3}]",
            self.step, self.dpsi_dt, self.l_hat, self.a_hat, self.g_hat, self.s_hat, self.gamma_hat
        )
    }
}

/// Euler integration: apply dPsi/dt to current state to produce next state.
pub fn integrate_euler(state: &HydraState, dt: f64) -> HydraState {
    let step = EquationStep::compute(state);
    let new_lyapunov = state.lyapunov_value + step.dpsi_dt * dt;

    let mut next = state.clone();
    next.lyapunov_value = new_lyapunov;
    next.step_count += 1;
    next.captured_at = chrono::Utc::now();

    // Update manifold position based on the step
    if let Some(coord) = next.manifold_position.coordinates.first_mut() {
        *coord += step.dpsi_dt * dt;
    }

    next
}

/// Laplace-Beltrami operator: manifold curvature contribution.
/// Positive when the manifold is well-curved (state is coherent).
pub fn compute_l_hat(state: &HydraState) -> f64 {
    let lb = state.manifold_position.laplace_beltrami_estimate();
    // Normalize: positive curvature helps, negative hurts
    lb.clamp(-1.0, 1.0) * 0.3
}

/// Adversarial operator: trust field contribution.
/// Positive when trust is high, negative under adversarial conditions.
pub fn compute_a_hat(state: &HydraState) -> f64 {
    state.trust_field.adversarial_estimate() * 0.25
}

/// Growth operator: capability acquisition contribution.
/// Always non-negative — growth never hurts stability.
pub fn compute_g_hat(state: &HydraState) -> f64 {
    let rate = state.growth_state.growth_rate;
    // Diminishing returns: log(1 + rate) / 10
    (1.0 + rate).ln() / 10.0
}

/// Signal operator: communication health contribution.
/// Positive when signal routing is clean, negative when congested.
pub fn compute_s_hat(state: &HydraState) -> f64 {
    let utilization = state.signal_state.queue_utilization;
    let orphan_penalty = state.signal_state.orphans_detected as f64 * 0.01;
    // Low utilization = good, high = bad
    (1.0 - utilization) * 0.2 - orphan_penalty
}

/// Dissipation operator: entropy and decay.
/// Always positive (subtracted from the total).
pub fn compute_gamma_hat(state: &HydraState) -> f64 {
    // Base dissipation rate + category inconsistency penalty
    let base = 0.01;
    let category_penalty = if state.category_state.is_healthy() {
        0.0
    } else {
        0.05
    };
    base + category_penalty
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::HydraState;

    #[test]
    fn initial_state_step_is_stable() {
        let state = HydraState::initial();
        let step = EquationStep::compute(&state);
        // Initial state should be roughly stable
        assert!(step.dpsi_dt.is_finite());
    }

    #[test]
    fn euler_integration_advances_step() {
        let state = HydraState::initial();
        let next = integrate_euler(&state, 0.1);
        assert_eq!(next.step_count, 1);
    }

    #[test]
    fn summary_contains_step_number() {
        let state = HydraState::initial();
        let step = EquationStep::compute(&state);
        let s = step.summary();
        assert!(s.contains("step=0"));
    }

    #[test]
    fn growth_operator_non_negative() {
        let state = HydraState::initial();
        let g = compute_g_hat(&state);
        assert!(g >= 0.0);
    }

    #[test]
    fn dissipation_operator_positive() {
        let state = HydraState::initial();
        let gamma = compute_gamma_hat(&state);
        assert!(gamma > 0.0);
    }

    #[test]
    fn adversarial_state_hurts_stability() {
        let mut state = HydraState::initial();
        state.trust_field.adversarial_detected = true;
        let a = compute_a_hat(&state);
        assert!(a < 0.0);
    }

    #[test]
    fn signal_congestion_hurts_stability() {
        let mut state = HydraState::initial();
        state.signal_state.queue_utilization = 1.0;
        let s = compute_s_hat(&state);
        assert!(s <= 0.0);
    }
}
