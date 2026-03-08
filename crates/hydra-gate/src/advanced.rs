//! Advanced gate features — conditionally uses private hydra-gate-advanced crate.
//! Public builds get safe stubs. Private builds get real implementations.

use hydra_core::types::Action;
use serde::{Deserialize, Serialize};

// === Re-export types (always available) ===

/// Result of a shadow simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimResult {
    pub simulated: bool,
    pub outcome: SimOutcome,
    pub side_effects: Vec<String>,
    pub rollback_possible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimOutcome {
    Safe,
    Warning { message: String },
    Dangerous { reason: String, severity: f64 },
}

impl SimResult {
    pub fn skip() -> Self {
        Self {
            simulated: false,
            outcome: SimOutcome::Safe,
            side_effects: vec![],
            rollback_possible: true,
        }
    }

    pub fn is_safe(&self) -> bool {
        matches!(self.outcome, SimOutcome::Safe)
    }
}

// === Feature-gated implementations ===

/// Run a shadow simulation of the action.
/// With `advanced-gate`: full simulation with side-effect prediction.
/// Without: returns `SimResult::skip()` (no prediction = allow).
#[cfg(feature = "advanced-gate")]
pub fn shadow_sim(action: &Action) -> SimResult {
    let result = hydra_gate_advanced::shadow_sim(action);
    // Convert from private type to public type
    SimResult {
        simulated: result.simulated,
        outcome: match result.outcome {
            hydra_gate_advanced::SimOutcome::Safe => SimOutcome::Safe,
            hydra_gate_advanced::SimOutcome::Warning { message } => SimOutcome::Warning { message },
            hydra_gate_advanced::SimOutcome::Dangerous { reason, severity } => {
                SimOutcome::Dangerous { reason, severity }
            }
        },
        side_effects: result.side_effects,
        rollback_possible: result.rollback_possible,
    }
}

#[cfg(not(feature = "advanced-gate"))]
pub fn shadow_sim(_action: &Action) -> SimResult {
    SimResult::skip()
}

/// Predict potential harm from an action.
/// With `advanced-gate`: heuristic harm analysis returning [0.0, 1.0].
/// Without: returns 0.0 (no prediction = allow).
#[cfg(feature = "advanced-gate")]
pub fn harm_predict(action: &Action) -> f64 {
    hydra_gate_advanced::harm_predict(action)
}

#[cfg(not(feature = "advanced-gate"))]
pub fn harm_predict(_action: &Action) -> f64 {
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_core::types::{ActionType, RiskLevel};

    fn test_action(target: &str) -> Action {
        Action::new(ActionType::Execute, target)
    }

    #[test]
    fn test_shadow_sim_returns_result() {
        let action = test_action("src/main.rs");
        let result = shadow_sim(&action);
        // In public build, should be skipped
        #[cfg(not(feature = "advanced-gate"))]
        {
            assert!(!result.simulated);
            assert!(result.is_safe());
        }
        // In private build, should be simulated
        #[cfg(feature = "advanced-gate")]
        {
            assert!(result.simulated);
        }
    }

    #[test]
    fn test_harm_predict_returns_score() {
        let action = test_action("src/main.rs");
        let score = harm_predict(&action);
        assert!(score >= 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_sim_result_skip() {
        let result = SimResult::skip();
        assert!(!result.simulated);
        assert!(result.is_safe());
        assert!(result.rollback_possible);
    }
}
