use serde::{Deserialize, Serialize};
use uuid::Uuid;

use hydra_core::error::HydraError;
use hydra_core::types::{Belief, CognitivePhase, Goal, TokenBudget};

use crate::config::CheckpointLevel;

/// Kernel run state — tracks what phase we're in, whether interrupted, etc.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KernelRunState {
    Idle,
    Running,
    Suspended,
    Recovering,
    Completed,
    Failed,
    Interrupted,
}

/// Serializable checkpoint for suspend/resume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: Uuid,
    pub phase: CognitivePhase,
    pub level: CheckpointLevel,
    pub context: serde_json::Value,
    pub goals: Vec<String>,
    pub budget: TokenBudget,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub partial_result: Option<serde_json::Value>,
}

impl Checkpoint {
    pub fn capture(
        phase: CognitivePhase,
        level: CheckpointLevel,
        context: serde_json::Value,
        goals: Vec<String>,
        budget: TokenBudget,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            phase,
            level,
            context,
            goals,
            budget,
            created_at: chrono::Utc::now(),
            partial_result: None,
        }
    }

    pub fn with_partial_result(mut self, result: serde_json::Value) -> Self {
        self.partial_result = Some(result);
        self
    }
}

/// Unified cognitive state — the kernel's mind at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveState {
    pub phase: CognitivePhase,
    pub context: serde_json::Value,
    pub goals: Vec<Goal>,
    pub budget: TokenBudget,
    pub beliefs: Vec<Belief>,
    pub run_state: KernelRunState,
}

impl CognitiveState {
    pub fn new(budget: TokenBudget) -> Self {
        Self {
            phase: CognitivePhase::Perceive,
            context: serde_json::Value::Null,
            goals: vec![],
            budget,
            beliefs: vec![],
            run_state: KernelRunState::Idle,
        }
    }

    pub fn with_budget(budget: TokenBudget) -> Self {
        Self::new(budget)
    }

    /// Transition to a new cognitive phase with validation
    pub fn transition_to(&mut self, target: CognitivePhase) -> Result<(), HydraError> {
        let valid = match self.phase {
            CognitivePhase::Perceive => target == CognitivePhase::Think,
            CognitivePhase::Think => {
                target == CognitivePhase::Decide || target == CognitivePhase::Act
            }
            CognitivePhase::Decide => target == CognitivePhase::Act,
            CognitivePhase::Act => {
                target == CognitivePhase::Learn || target == CognitivePhase::Think
            }
            CognitivePhase::Learn => target == CognitivePhase::Perceive,
        };
        if valid {
            self.phase = target;
            Ok(())
        } else {
            Err(HydraError::Internal(format!(
                "Invalid phase transition: {:?} -> {:?}",
                self.phase, target
            )))
        }
    }

    /// Create a checkpoint from the current state
    pub fn checkpoint(&self, level: CheckpointLevel) -> Checkpoint {
        Checkpoint::capture(
            self.phase,
            level,
            self.context.clone(),
            self.goals.iter().map(|g| g.target.clone()).collect(),
            self.budget.clone(),
        )
    }

    /// Restore state from a checkpoint
    pub fn restore_from(checkpoint: &Checkpoint) -> Self {
        Self {
            phase: checkpoint.phase,
            context: checkpoint.context.clone(),
            goals: vec![],
            budget: checkpoint.budget.clone(),
            beliefs: vec![],
            run_state: KernelRunState::Suspended,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cognitive_state_new_starts_at_perceive() {
        let state = CognitiveState::new(TokenBudget::new(1_000));
        assert_eq!(state.phase, CognitivePhase::Perceive);
        assert_eq!(state.run_state, KernelRunState::Idle);
        assert!(state.goals.is_empty());
        assert!(state.beliefs.is_empty());
    }

    #[test]
    fn test_valid_transition_perceive_to_think() {
        let mut state = CognitiveState::new(TokenBudget::new(1_000));
        assert!(state.transition_to(CognitivePhase::Think).is_ok());
        assert_eq!(state.phase, CognitivePhase::Think);
    }

    #[test]
    fn test_valid_transition_think_to_decide() {
        let mut state = CognitiveState::new(TokenBudget::new(1_000));
        state.transition_to(CognitivePhase::Think).unwrap();
        assert!(state.transition_to(CognitivePhase::Decide).is_ok());
        assert_eq!(state.phase, CognitivePhase::Decide);
    }

    #[test]
    fn test_valid_transition_think_to_act() {
        let mut state = CognitiveState::new(TokenBudget::new(1_000));
        state.transition_to(CognitivePhase::Think).unwrap();
        assert!(state.transition_to(CognitivePhase::Act).is_ok());
    }

    #[test]
    fn test_valid_transition_decide_to_act() {
        let mut state = CognitiveState::new(TokenBudget::new(1_000));
        state.transition_to(CognitivePhase::Think).unwrap();
        state.transition_to(CognitivePhase::Decide).unwrap();
        assert!(state.transition_to(CognitivePhase::Act).is_ok());
    }

    #[test]
    fn test_valid_transition_act_to_learn() {
        let mut state = CognitiveState::new(TokenBudget::new(1_000));
        state.transition_to(CognitivePhase::Think).unwrap();
        state.transition_to(CognitivePhase::Act).unwrap();
        assert!(state.transition_to(CognitivePhase::Learn).is_ok());
    }

    #[test]
    fn test_valid_transition_act_to_think_loop() {
        let mut state = CognitiveState::new(TokenBudget::new(1_000));
        state.transition_to(CognitivePhase::Think).unwrap();
        state.transition_to(CognitivePhase::Act).unwrap();
        assert!(state.transition_to(CognitivePhase::Think).is_ok());
    }

    #[test]
    fn test_valid_transition_learn_to_perceive() {
        let mut state = CognitiveState::new(TokenBudget::new(1_000));
        state.transition_to(CognitivePhase::Think).unwrap();
        state.transition_to(CognitivePhase::Act).unwrap();
        state.transition_to(CognitivePhase::Learn).unwrap();
        assert!(state.transition_to(CognitivePhase::Perceive).is_ok());
    }

    #[test]
    fn test_invalid_transition_perceive_to_act() {
        let mut state = CognitiveState::new(TokenBudget::new(1_000));
        let result = state.transition_to(CognitivePhase::Act);
        assert!(result.is_err());
        // Phase should remain unchanged
        assert_eq!(state.phase, CognitivePhase::Perceive);
    }

    #[test]
    fn test_invalid_transition_perceive_to_learn() {
        let mut state = CognitiveState::new(TokenBudget::new(1_000));
        assert!(state.transition_to(CognitivePhase::Learn).is_err());
    }

    #[test]
    fn test_invalid_transition_decide_to_perceive() {
        let mut state = CognitiveState::new(TokenBudget::new(1_000));
        state.transition_to(CognitivePhase::Think).unwrap();
        state.transition_to(CognitivePhase::Decide).unwrap();
        assert!(state.transition_to(CognitivePhase::Perceive).is_err());
    }

    #[test]
    fn test_checkpoint_capture_preserves_phase() {
        let cp = Checkpoint::capture(
            CognitivePhase::Decide,
            CheckpointLevel::Full,
            json!({"key": "value"}),
            vec!["goal1".into()],
            TokenBudget::new(5_000),
        );
        assert_eq!(cp.phase, CognitivePhase::Decide);
        assert_eq!(cp.level, CheckpointLevel::Full);
        assert_eq!(cp.goals, vec!["goal1".to_string()]);
        assert_eq!(cp.context, json!({"key": "value"}));
    }

    #[test]
    fn test_checkpoint_has_unique_id() {
        let cp1 = Checkpoint::capture(
            CognitivePhase::Act,
            CheckpointLevel::Atomic,
            json!({}),
            vec![],
            TokenBudget::new(1_000),
        );
        let cp2 = Checkpoint::capture(
            CognitivePhase::Act,
            CheckpointLevel::Atomic,
            json!({}),
            vec![],
            TokenBudget::new(1_000),
        );
        assert_ne!(cp1.id, cp2.id);
    }

    #[test]
    fn test_checkpoint_with_partial_result() {
        let cp = Checkpoint::capture(
            CognitivePhase::Act,
            CheckpointLevel::Atomic,
            json!({}),
            vec![],
            TokenBudget::new(1_000),
        )
        .with_partial_result(json!({"partial": true}));
        assert_eq!(cp.partial_result, Some(json!({"partial": true})));
    }

    #[test]
    fn test_checkpoint_default_has_no_partial_result() {
        let cp = Checkpoint::capture(
            CognitivePhase::Think,
            CheckpointLevel::Full,
            json!({}),
            vec![],
            TokenBudget::new(1_000),
        );
        assert!(cp.partial_result.is_none());
    }

    #[test]
    fn test_restore_from_checkpoint() {
        let cp = Checkpoint::capture(
            CognitivePhase::Decide,
            CheckpointLevel::Full,
            json!({"restored": true}),
            vec!["g1".into(), "g2".into()],
            TokenBudget::new(3_000),
        );
        let state = CognitiveState::restore_from(&cp);
        assert_eq!(state.phase, CognitivePhase::Decide);
        assert_eq!(state.context, json!({"restored": true}));
        assert_eq!(state.run_state, KernelRunState::Suspended);
        assert_eq!(state.budget.total, 3_000);
    }

    #[test]
    fn test_cognitive_state_checkpoint_roundtrip() {
        let mut state = CognitiveState::new(TokenBudget::new(8_000));
        state.context = json!({"round": "trip"});
        state.transition_to(CognitivePhase::Think).unwrap();

        let cp = state.checkpoint(CheckpointLevel::Full);
        assert_eq!(cp.phase, CognitivePhase::Think);
        assert_eq!(cp.context, json!({"round": "trip"}));

        let restored = CognitiveState::restore_from(&cp);
        assert_eq!(restored.phase, CognitivePhase::Think);
        assert_eq!(restored.context, json!({"round": "trip"}));
    }

    #[test]
    fn test_kernel_run_state_serialization() {
        let json_str = serde_json::to_string(&KernelRunState::Running).unwrap();
        assert_eq!(json_str, "\"running\"");
        let deserialized: KernelRunState = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized, KernelRunState::Running);
    }

    #[test]
    fn test_with_budget_alias() {
        let state = CognitiveState::with_budget(TokenBudget::new(42));
        assert_eq!(state.budget.total, 42);
        assert_eq!(state.phase, CognitivePhase::Perceive);
    }
}
