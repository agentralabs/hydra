use serde::{Deserialize, Serialize};

use crate::belief::{Belief, BeliefSource};

/// A detected conflict between beliefs
#[derive(Debug, Clone)]
pub struct Conflict {
    pub existing: Belief,
    pub incoming: Belief,
    pub similarity: f32,
}

/// Strategy for resolving conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    NewerWins,
    HigherConfidence,
    UserStatedWins,
    AskUser,
}

/// Result of conflict resolution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    KeepOld,
    KeepNew,
    Merge,
    AskUser,
}

/// Resolve a conflict using the given strategy (deterministic)
pub fn resolve_conflict(conflict: &Conflict, strategy: ConflictStrategy) -> Resolution {
    match strategy {
        ConflictStrategy::NewerWins => Resolution::KeepNew,
        ConflictStrategy::HigherConfidence => {
            if conflict.incoming.confidence >= conflict.existing.confidence {
                Resolution::KeepNew
            } else {
                Resolution::KeepOld
            }
        }
        ConflictStrategy::UserStatedWins => {
            match (conflict.incoming.source, conflict.existing.source) {
                (BeliefSource::Corrected, _) => Resolution::KeepNew,
                (BeliefSource::UserStated, BeliefSource::Inferred) => Resolution::KeepNew,
                (BeliefSource::Inferred, BeliefSource::UserStated) => Resolution::KeepOld,
                (BeliefSource::Inferred, BeliefSource::Corrected) => Resolution::KeepOld,
                _ => {
                    // Same source — use confidence
                    if conflict.incoming.confidence >= conflict.existing.confidence {
                        Resolution::KeepNew
                    } else {
                        Resolution::KeepOld
                    }
                }
            }
        }
        ConflictStrategy::AskUser => Resolution::AskUser,
    }
}
