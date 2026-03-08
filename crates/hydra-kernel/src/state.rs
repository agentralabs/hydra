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
