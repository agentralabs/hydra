//! ActionHorizon — what Hydra can currently do and affect.
//! Expands with synthesized capabilities, loaded skills, and genome entries.
//! Never contracts. Knowledge of action persists forever.

use crate::constants::*;
use serde::{Deserialize, Serialize};

/// What contributes to action horizon expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionExpansion {
    /// New capability synthesized by hydra-generative.
    CapabilitySynthesized {
        /// Name of the synthesized capability.
        name: String,
    },
    /// New skill loaded with action capabilities.
    SkillLoaded {
        /// Name of the loaded skill.
        skill_name: String,
        /// Number of capabilities the skill provides.
        capability_count: usize,
    },
    /// New genome entry — proven approach now available.
    GenomeEntry {
        /// Number of new genome entries.
        count_delta: u64,
    },
    /// New environment profiled — can now execute there.
    EnvironmentProfiled {
        /// Name of the profiled environment.
        env_name: String,
    },
    /// New sister connected — new action domain available.
    SisterConnected {
        /// Name of the connected sister.
        sister_name: String,
    },
}

/// The action horizon — what Hydra can currently do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHorizon {
    /// Current action value in [0.0, HORIZON_MAX].
    pub value: f64,
    /// Total number of expansions applied.
    pub expansion_count: u64,
    /// Total capabilities synthesized or loaded.
    pub capabilities_total: u64,
    /// Total skills loaded.
    pub skills_loaded: u64,
    /// Total environments known.
    pub environments_known: u64,
}

impl ActionHorizon {
    /// Create a new action horizon at the initial value.
    pub fn new() -> Self {
        Self {
            value: ACTION_HORIZON_INITIAL,
            expansion_count: 0,
            capabilities_total: 0,
            skills_loaded: 0,
            environments_known: 0,
        }
    }

    /// Return the current action horizon value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Return the total number of expansions applied.
    pub fn expansion_count(&self) -> u64 {
        self.expansion_count
    }

    /// Expand the action horizon by the given expansion type.
    ///
    /// Returns the delta applied. The horizon is capped at HORIZON_MAX.
    pub fn expand(&mut self, reason: ActionExpansion) -> f64 {
        let before = self.value;

        let delta = match &reason {
            ActionExpansion::CapabilitySynthesized { .. } => {
                self.capabilities_total += 1;
                ACTION_SYNTHESIS_FACTOR
            }
            ActionExpansion::SkillLoaded {
                capability_count, ..
            } => {
                self.capabilities_total += *capability_count as u64;
                self.skills_loaded += 1;
                *capability_count as f64 * 0.001
            }
            ActionExpansion::GenomeEntry { count_delta } => {
                *count_delta as f64 * ACTION_SYNTHESIS_FACTOR * 0.5
            }
            ActionExpansion::EnvironmentProfiled { .. } => {
                self.environments_known += 1;
                0.004
            }
            ActionExpansion::SisterConnected { .. } => 0.015,
        };

        self.value = (self.value + delta.max(HORIZON_MIN_DELTA)).min(HORIZON_MAX);
        self.expansion_count += 1;
        self.value - before
    }
}

impl Default for ActionHorizon {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_synthesized_expands() {
        let mut h = ActionHorizon::new();
        let before = h.value;
        h.expand(ActionExpansion::CapabilitySynthesized {
            name: "risk-constrained-deploy".into(),
        });
        assert!(h.value > before);
        assert_eq!(h.capabilities_total, 1);
    }

    #[test]
    fn skill_loaded_expands_proportionally() {
        let mut h = ActionHorizon::new();
        let before = h.value;
        h.expand(ActionExpansion::SkillLoaded {
            skill_name: "finance".into(),
            capability_count: 20,
        });
        assert!(h.value > before);
        assert_eq!(h.skills_loaded, 1);
    }

    #[test]
    fn horizon_never_exceeds_max() {
        let mut h = ActionHorizon::new();
        for _ in 0..10_000 {
            h.expand(ActionExpansion::CapabilitySynthesized {
                name: "x".into(),
            });
        }
        assert!(h.value <= HORIZON_MAX);
    }
}
