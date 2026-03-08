//! Replay — replay from checkpoint with modifications.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::checkpoint::Checkpoint;

/// A modification to apply during replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayModification {
    pub key: String,
    pub action: ModAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModAction {
    Set(serde_json::Value),
    Remove,
    Transform(String), // Description of transformation
}

/// Result of a replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    pub success: bool,
    pub final_state: HashMap<String, serde_json::Value>,
    pub modifications_applied: usize,
    pub original_checkpoint: String,
}

/// Replays from a checkpoint with optional modifications
pub struct Replayer;

impl Replayer {
    /// Replay a checkpoint, applying modifications
    pub fn replay(checkpoint: &Checkpoint, modifications: &[ReplayModification]) -> ReplayResult {
        let mut state = checkpoint.state.clone();
        let mut applied = 0;

        for modification in modifications {
            match &modification.action {
                ModAction::Set(value) => {
                    state.insert(modification.key.clone(), value.clone());
                    applied += 1;
                }
                ModAction::Remove => {
                    if state.remove(&modification.key).is_some() {
                        applied += 1;
                    }
                }
                ModAction::Transform(_desc) => {
                    // In production: apply transformation function
                    // For now: mark as applied if key exists
                    if state.contains_key(&modification.key) {
                        applied += 1;
                    }
                }
            }
        }

        ReplayResult {
            success: true,
            final_state: state,
            modifications_applied: applied,
            original_checkpoint: checkpoint.id.clone(),
        }
    }

    /// Replay and compare with another checkpoint
    pub fn replay_and_diff(
        checkpoint: &Checkpoint,
        modifications: &[ReplayModification],
        compare_with: &Checkpoint,
    ) -> (ReplayResult, super::checkpoint::CheckpointDiff) {
        let result = Self::replay(checkpoint, modifications);

        let replayed_cp =
            Checkpoint::create("replayed", result.final_state.clone(), Some(&checkpoint.id));
        let diff = replayed_cp.diff(compare_with);

        (result, diff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_modified() {
        let state = HashMap::from([
            ("a".into(), serde_json::json!(1)),
            ("b".into(), serde_json::json!(2)),
        ]);
        let cp = Checkpoint::create("orig", state, None);

        let mods = vec![
            ReplayModification {
                key: "a".into(),
                action: ModAction::Set(serde_json::json!(99)),
            },
            ReplayModification {
                key: "c".into(),
                action: ModAction::Set(serde_json::json!(3)),
            },
            ReplayModification {
                key: "b".into(),
                action: ModAction::Remove,
            },
        ];

        let result = Replayer::replay(&cp, &mods);
        assert!(result.success);
        assert_eq!(result.modifications_applied, 3);
        assert_eq!(result.final_state["a"], serde_json::json!(99));
        assert_eq!(result.final_state["c"], serde_json::json!(3));
        assert!(!result.final_state.contains_key("b"));
    }

    #[test]
    fn test_resurrection_from_receipt() {
        // Simulate: build state from a series of "receipt" entries
        let mut state = HashMap::new();
        let receipts = vec![
            ("memory.add", serde_json::json!({"entry": "hello"})),
            ("identity.set", serde_json::json!({"name": "hydra"})),
            ("beliefs.update", serde_json::json!({"confidence": 0.9})),
        ];

        for (key, value) in receipts {
            state.insert(key.into(), value);
        }

        let cp = Checkpoint::create("receipt-state", state, None);
        assert_eq!(cp.state.len(), 3);

        // Replay with modification
        let mods = vec![ReplayModification {
            key: "beliefs.update".into(),
            action: ModAction::Set(serde_json::json!({"confidence": 0.5})),
        }];
        let result = Replayer::replay(&cp, &mods);
        assert_eq!(
            result.final_state["beliefs.update"],
            serde_json::json!({"confidence": 0.5})
        );
    }
}
