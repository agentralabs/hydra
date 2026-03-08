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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::{Belief, BeliefCategory};

    fn make_conflict(incoming_source: BeliefSource, existing_source: BeliefSource) -> Conflict {
        let existing = Belief::new(BeliefCategory::Fact, "test subject", "old content", existing_source);
        let incoming = Belief::new(BeliefCategory::Fact, "test subject", "new content", incoming_source);
        Conflict {
            existing,
            incoming,
            similarity: 1.0,
        }
    }

    #[test]
    fn newer_wins_always_keeps_new() {
        let conflict = make_conflict(BeliefSource::Inferred, BeliefSource::UserStated);
        assert_eq!(resolve_conflict(&conflict, ConflictStrategy::NewerWins), Resolution::KeepNew);
    }

    #[test]
    fn higher_confidence_keeps_higher() {
        let conflict = make_conflict(BeliefSource::Corrected, BeliefSource::Inferred);
        // Corrected = 0.99, Inferred = 0.6
        assert_eq!(
            resolve_conflict(&conflict, ConflictStrategy::HigherConfidence),
            Resolution::KeepNew
        );
    }

    #[test]
    fn higher_confidence_keeps_old_when_existing_higher() {
        let conflict = make_conflict(BeliefSource::Inferred, BeliefSource::Corrected);
        // Inferred = 0.6, Corrected = 0.99
        assert_eq!(
            resolve_conflict(&conflict, ConflictStrategy::HigherConfidence),
            Resolution::KeepOld
        );
    }

    #[test]
    fn user_stated_wins_corrected_beats_all() {
        let conflict = make_conflict(BeliefSource::Corrected, BeliefSource::UserStated);
        assert_eq!(
            resolve_conflict(&conflict, ConflictStrategy::UserStatedWins),
            Resolution::KeepNew
        );
    }

    #[test]
    fn user_stated_wins_user_beats_inferred() {
        let conflict = make_conflict(BeliefSource::UserStated, BeliefSource::Inferred);
        assert_eq!(
            resolve_conflict(&conflict, ConflictStrategy::UserStatedWins),
            Resolution::KeepNew
        );
    }

    #[test]
    fn user_stated_wins_inferred_loses_to_user() {
        let conflict = make_conflict(BeliefSource::Inferred, BeliefSource::UserStated);
        assert_eq!(
            resolve_conflict(&conflict, ConflictStrategy::UserStatedWins),
            Resolution::KeepOld
        );
    }

    #[test]
    fn user_stated_wins_inferred_loses_to_corrected() {
        let conflict = make_conflict(BeliefSource::Inferred, BeliefSource::Corrected);
        assert_eq!(
            resolve_conflict(&conflict, ConflictStrategy::UserStatedWins),
            Resolution::KeepOld
        );
    }

    #[test]
    fn ask_user_strategy() {
        let conflict = make_conflict(BeliefSource::Inferred, BeliefSource::Inferred);
        assert_eq!(resolve_conflict(&conflict, ConflictStrategy::AskUser), Resolution::AskUser);
    }
}
