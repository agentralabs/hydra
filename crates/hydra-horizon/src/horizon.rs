//! Horizon — the unified perception + action measurement.

use crate::{
    action::{ActionExpansion, ActionHorizon},
    errors::HorizonError,
    perception::{PerceptionExpansion, PerceptionHorizon},
};
use serde::{Deserialize, Serialize};

/// The complete horizon state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Horizon {
    /// The perception horizon.
    pub perception: PerceptionHorizon,
    /// The action horizon.
    pub action: ActionHorizon,
}

impl Horizon {
    /// Create a new horizon with both sub-horizons at initial values.
    pub fn new() -> Self {
        Self {
            perception: PerceptionHorizon::new(),
            action: ActionHorizon::new(),
        }
    }

    /// Combined horizon score (geometric mean of perception + action).
    pub fn combined(&self) -> f64 {
        (self.perception.value * self.action.value).sqrt()
    }

    /// Expand the perception horizon.
    pub fn expand_perception(
        &mut self,
        reason: PerceptionExpansion,
    ) -> Result<f64, HorizonError> {
        self.perception.expand(reason)
    }

    /// Expand the action horizon.
    pub fn expand_action(&mut self, reason: ActionExpansion) -> f64 {
        self.action.expand(reason)
    }

    /// Status line for TUI display.
    pub fn status_line(&self) -> String {
        format!(
            "Horizon: perception={:.3} action={:.3} combined={:.3}",
            self.perception.value,
            self.action.value,
            self.combined(),
        )
    }
}

impl Default for Horizon {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ActionExpansion;
    use crate::perception::PerceptionExpansion;

    #[test]
    fn combined_is_geometric_mean() {
        let mut h = Horizon::new();
        h.expand_perception(PerceptionExpansion::SisterConnected {
            sister_name: "memory".into(),
        })
        .unwrap();
        h.expand_action(ActionExpansion::CapabilitySynthesized {
            name: "test".into(),
        });
        let expected = (h.perception.value * h.action.value).sqrt();
        assert!((h.combined() - expected).abs() < 1e-10);
    }

    #[test]
    fn horizon_both_expand_from_single_event() {
        let mut h = Horizon::new();
        let p_before = h.perception.value;
        let a_before = h.action.value;
        h.expand_perception(PerceptionExpansion::SisterConnected {
            sister_name: "forge".into(),
        })
        .unwrap();
        h.expand_action(ActionExpansion::SisterConnected {
            sister_name: "forge".into(),
        });
        assert!(h.perception.value > p_before);
        assert!(h.action.value > a_before);
    }

    #[test]
    fn status_line_contains_fields() {
        let h = Horizon::new();
        let line = h.status_line();
        assert!(line.contains("perception="));
        assert!(line.contains("action="));
        assert!(line.contains("combined="));
    }
}
