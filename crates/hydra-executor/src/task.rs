//! TaskState — the relentless task state machine.
//! FAILED does not exist as a state. Ever.

use serde::{Deserialize, Serialize};

/// The approach types Hydra cycles through before HardDenied.
/// Order matters — kernel cycles through in sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApproachType {
    DirectExecution,
    AlternativeTooling,
    EnvironmentAdapt,
    ProtocolSwitch,
    DecompositionRetry,
    GenomeConsult,
    CartographyConsult,
    AntifragileConsult,
    AgentDelegation,
    GenerativeSynthesis,
    PlasticityReshape,
    PatienceStrategy,
    EscalateToSwarm,
}

impl ApproachType {
    /// All approaches in escalation order.
    pub fn all_in_order() -> Vec<ApproachType> {
        vec![
            Self::DirectExecution,
            Self::AlternativeTooling,
            Self::EnvironmentAdapt,
            Self::ProtocolSwitch,
            Self::DecompositionRetry,
            Self::GenomeConsult,
            Self::CartographyConsult,
            Self::AntifragileConsult,
            Self::AgentDelegation,
            Self::GenerativeSynthesis,
            Self::PlasticityReshape,
            Self::PatienceStrategy,
            Self::EscalateToSwarm,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::DirectExecution => "direct",
            Self::AlternativeTooling => "alternative-tooling",
            Self::EnvironmentAdapt => "environment-adapt",
            Self::ProtocolSwitch => "protocol-switch",
            Self::DecompositionRetry => "decompose-retry",
            Self::GenomeConsult => "genome-consult",
            Self::CartographyConsult => "cartography-consult",
            Self::AntifragileConsult => "antifragile-consult",
            Self::AgentDelegation => "agent-delegation",
            Self::GenerativeSynthesis => "generative-synthesis",
            Self::PlasticityReshape => "plasticity-reshape",
            Self::PatienceStrategy => "patience",
            Self::EscalateToSwarm => "swarm-escalation",
        }
    }
}

/// Task state — FAILED does not exist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskState {
    /// Working on it now.
    Active { approach: ApproachType },
    /// Obstacle encountered — computing alternate path.
    Blocked {
        reason: String,
        approach: ApproachType,
    },
    /// Trying a different approach.
    Rerouting {
        attempt: u32,
        next_approach: ApproachType,
    },
    /// Spawning a specialist fleet agent.
    EscalatingToAgent { agent_type: String },
    /// Waiting for a condition to change — NOT giving up.
    Suspended {
        condition: String,
        retry_after_seconds: u64,
    },
    /// Done. Receipted. Permanent.
    Complete { receipt_id: String },
    /// One of three hard stops, evidenced and receipted.
    HardDenied {
        evidence: String,
        receipt_id: String,
    },
    // FAILED: deliberately absent. It does not exist.
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete { .. } | Self::HardDenied { .. })
    }

    pub fn is_hard_denied(&self) -> bool {
        matches!(self, Self::HardDenied { .. })
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Active { .. } => "active",
            Self::Blocked { .. } => "blocked",
            Self::Rerouting { .. } => "rerouting",
            Self::EscalatingToAgent { .. } => "escalating",
            Self::Suspended { .. } => "suspended",
            Self::Complete { .. } => "complete",
            Self::HardDenied { .. } => "hard-denied",
        }
    }
}

/// One attempt record — what was tried and what happened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub attempt_number: u32,
    pub approach: ApproachType,
    pub obstacle: Option<String>,
    pub resolution: Option<String>,
    pub next_approach: Option<ApproachType>,
    pub duration_ms: u64,
    pub receipt_id: String,
}

/// The complete task record — everything about one execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub action_id: String,
    pub intent: String,
    pub state: TaskState,
    pub attempts: Vec<AttemptRecord>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl TaskRecord {
    pub fn new(action_id: impl Into<String>, intent: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action_id: action_id.into(),
            intent: intent.into(),
            state: TaskState::Active {
                approach: ApproachType::DirectExecution,
            },
            attempts: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn attempt_count(&self) -> u32 {
        self.attempts.len() as u32
    }

    pub fn transition(&mut self, new_state: TaskState) {
        self.state = new_state;
        self.updated_at = chrono::Utc::now();
    }

    pub fn add_attempt(&mut self, attempt: AttemptRecord) {
        self.attempts.push(attempt);
        self.updated_at = chrono::Utc::now();
    }

    pub fn next_approach(&self) -> Option<ApproachType> {
        let all = ApproachType::all_in_order();
        let current = match &self.state {
            TaskState::Active { approach } => approach,
            TaskState::Blocked { approach, .. } => approach,
            _ => return None,
        };
        let idx = all.iter().position(|a| a == current)?;
        all.into_iter().nth(idx + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_state_does_not_exist() {
        let states = vec![
            TaskState::Active {
                approach: ApproachType::DirectExecution,
            },
            TaskState::Blocked {
                reason: "test".into(),
                approach: ApproachType::DirectExecution,
            },
            TaskState::Rerouting {
                attempt: 1,
                next_approach: ApproachType::AlternativeTooling,
            },
            TaskState::EscalatingToAgent {
                agent_type: "specialist".into(),
            },
            TaskState::Suspended {
                condition: "waiting".into(),
                retry_after_seconds: 60,
            },
            TaskState::Complete {
                receipt_id: "r1".into(),
            },
            TaskState::HardDenied {
                evidence: "test".into(),
                receipt_id: "r2".into(),
            },
        ];
        for s in &states {
            assert_ne!(s.label(), "failed");
        }
    }

    #[test]
    fn only_complete_and_hard_denied_are_terminal() {
        assert!(TaskState::Complete {
            receipt_id: "r".into()
        }
        .is_terminal());
        assert!(TaskState::HardDenied {
            evidence: "e".into(),
            receipt_id: "r".into()
        }
        .is_terminal());
        assert!(!TaskState::Active {
            approach: ApproachType::DirectExecution
        }
        .is_terminal());
        assert!(!TaskState::Blocked {
            reason: "b".into(),
            approach: ApproachType::DirectExecution
        }
        .is_terminal());
        assert!(!TaskState::EscalatingToAgent {
            agent_type: "t".into()
        }
        .is_terminal());
        assert!(!TaskState::Suspended {
            condition: "c".into(),
            retry_after_seconds: 10
        }
        .is_terminal());
    }

    #[test]
    fn approach_type_has_13_variants() {
        let approaches = ApproachType::all_in_order();
        assert_eq!(approaches.len(), 13);
    }

    #[test]
    fn next_approach_cycles_through_all() {
        let task = TaskRecord::new("test-action", "test intent");
        let next = task.next_approach();
        assert_eq!(next, Some(ApproachType::AlternativeTooling));
    }

    #[test]
    fn attempt_count_tracks_correctly() {
        let mut task = TaskRecord::new("action", "intent");
        assert_eq!(task.attempt_count(), 0);
        task.add_attempt(AttemptRecord {
            attempt_number: 1,
            approach: ApproachType::DirectExecution,
            obstacle: None,
            resolution: Some("succeeded".into()),
            next_approach: None,
            duration_ms: 50,
            receipt_id: "r1".into(),
        });
        assert_eq!(task.attempt_count(), 1);
    }
}
