//! Degradation actions — what to do when levels change.

use serde::{Deserialize, Serialize};

use super::manager::DegradationLevel;

/// An action taken in response to a degradation level change
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationAction {
    /// Disable shadow simulation (advanced gate)
    DisableShadowSim,
    /// Reduce cache size by a percentage
    ReduceCache { percent: u32 },
    /// Unload local LLM models
    UnloadLocalModels,
    /// Unload sister connections
    UnloadSisters,
    /// Switch to cheapest model only (Haiku)
    HaikuOnly,
    /// Pause new runs
    PauseNewRuns,
    /// Run aggressive garbage collection
    AggressiveGc,
    /// Resume normal operations
    ResumeNormal,
    /// Re-enable shadow simulation
    EnableShadowSim,
    /// Restore full cache
    RestoreCache,
    /// Reload local models
    ReloadLocalModels,
    /// Reconnect sisters
    ReconnectSisters,
    /// Restore model selection
    RestoreModels,
    /// Resume accepting runs
    ResumeRuns,
}

impl DegradationAction {
    /// Get the human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::DisableShadowSim => "Disable shadow simulation",
            Self::ReduceCache { .. } => "Reduce cache size",
            Self::UnloadLocalModels => "Unload local LLM models",
            Self::UnloadSisters => "Disconnect sister MCP servers",
            Self::HaikuOnly => "Switch to Haiku-only mode",
            Self::PauseNewRuns => "Pause new run submissions",
            Self::AggressiveGc => "Run aggressive garbage collection",
            Self::ResumeNormal => "Resume normal operations",
            Self::EnableShadowSim => "Re-enable shadow simulation",
            Self::RestoreCache => "Restore full cache",
            Self::ReloadLocalModels => "Reload local LLM models",
            Self::ReconnectSisters => "Reconnect sister MCP servers",
            Self::RestoreModels => "Restore full model selection",
            Self::ResumeRuns => "Resume accepting runs",
        }
    }
}

/// Get the actions to take when transitioning to a new degradation level
pub fn actions_for_transition(
    from: DegradationLevel,
    to: DegradationLevel,
) -> Vec<DegradationAction> {
    if from == to {
        return vec![];
    }

    if to > from {
        // Degrading — apply restrictions
        actions_to_degrade(from, to)
    } else {
        // Recovering — lift restrictions
        actions_to_recover(from, to)
    }
}

fn actions_to_degrade(from: DegradationLevel, to: DegradationLevel) -> Vec<DegradationAction> {
    let mut actions = Vec::new();

    // Normal → Reduced
    if from < DegradationLevel::Reduced && to >= DegradationLevel::Reduced {
        actions.push(DegradationAction::DisableShadowSim);
        actions.push(DegradationAction::ReduceCache { percent: 50 });
    }

    // Reduced → Minimal
    if from < DegradationLevel::Minimal && to >= DegradationLevel::Minimal {
        actions.push(DegradationAction::UnloadLocalModels);
        actions.push(DegradationAction::UnloadSisters);
        actions.push(DegradationAction::HaikuOnly);
    }

    // Minimal → Emergency
    if from < DegradationLevel::Emergency && to >= DegradationLevel::Emergency {
        actions.push(DegradationAction::PauseNewRuns);
        actions.push(DegradationAction::AggressiveGc);
    }

    actions
}

fn actions_to_recover(from: DegradationLevel, to: DegradationLevel) -> Vec<DegradationAction> {
    let mut actions = Vec::new();

    // Emergency → Minimal
    if from >= DegradationLevel::Emergency && to < DegradationLevel::Emergency {
        actions.push(DegradationAction::ResumeRuns);
    }

    // Minimal → Reduced
    if from >= DegradationLevel::Minimal && to < DegradationLevel::Minimal {
        actions.push(DegradationAction::ReloadLocalModels);
        actions.push(DegradationAction::ReconnectSisters);
        actions.push(DegradationAction::RestoreModels);
    }

    // Reduced → Normal
    if from >= DegradationLevel::Reduced && to < DegradationLevel::Reduced {
        actions.push(DegradationAction::EnableShadowSim);
        actions.push(DegradationAction::RestoreCache);
        actions.push(DegradationAction::ResumeNormal);
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_level_no_actions() {
        let actions = actions_for_transition(DegradationLevel::Normal, DegradationLevel::Normal);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_normal_to_reduced() {
        let actions = actions_for_transition(DegradationLevel::Normal, DegradationLevel::Reduced);
        assert!(actions.contains(&DegradationAction::DisableShadowSim));
        assert!(actions.contains(&DegradationAction::ReduceCache { percent: 50 }));
        assert!(!actions.contains(&DegradationAction::PauseNewRuns));
    }

    #[test]
    fn test_normal_to_emergency_all_actions() {
        let actions = actions_for_transition(DegradationLevel::Normal, DegradationLevel::Emergency);
        // Should include actions from all intermediate levels
        assert!(actions.contains(&DegradationAction::DisableShadowSim));
        assert!(actions.contains(&DegradationAction::UnloadLocalModels));
        assert!(actions.contains(&DegradationAction::PauseNewRuns));
        assert!(actions.contains(&DegradationAction::AggressiveGc));
    }

    #[test]
    fn test_emergency_to_normal_recovery() {
        let actions = actions_for_transition(DegradationLevel::Emergency, DegradationLevel::Normal);
        assert!(actions.contains(&DegradationAction::ResumeRuns));
        assert!(actions.contains(&DegradationAction::ReloadLocalModels));
        assert!(actions.contains(&DegradationAction::EnableShadowSim));
        assert!(actions.contains(&DegradationAction::ResumeNormal));
    }

    #[test]
    fn test_action_descriptions() {
        assert!(!DegradationAction::PauseNewRuns.description().is_empty());
        assert!(!DegradationAction::AggressiveGc.description().is_empty());
    }
}
