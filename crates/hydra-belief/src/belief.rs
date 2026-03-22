//! Core belief types and operations.

use crate::constants::{BELIEF_CONFIDENCE_MAX, BELIEF_CONFIDENCE_MIN};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Policy controlling how a belief can be revised.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevisionPolicy {
    /// Can be freely revised up or down.
    Standard,
    /// Can only be revised upward (capability beliefs).
    Protected,
    /// Cannot be revised at all.
    Immutable,
}

/// Category of a belief, determining its semantics and revision rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BeliefCategory {
    /// Beliefs about the state of the world.
    World,
    /// Beliefs about Hydra's own capabilities.
    Capability,
    /// Beliefs about trust levels of entities.
    Trust,
    /// Beliefs about temporal patterns and sequences.
    Temporal,
    /// Beliefs about security posture and threats.
    Security,
    /// Domain-specific beliefs.
    Domain(String),
}

/// A single belief in Hydra's belief set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    /// Unique identifier.
    pub id: String,
    /// The proposition this belief represents.
    pub proposition: String,
    /// Confidence in this belief (0.0 to 1.0).
    pub confidence: f64,
    /// The category of this belief.
    pub category: BeliefCategory,
    /// Revision policy.
    pub policy: RevisionPolicy,
    /// When this belief was created.
    pub created_at: DateTime<Utc>,
    /// When this belief was last revised.
    pub revised_at: DateTime<Utc>,
    /// Number of times this belief has been revised.
    pub revision_count: u32,
}

impl Belief {
    /// Create a new belief with the given proposition and confidence.
    pub fn new(
        proposition: impl Into<String>,
        confidence: f64,
        category: BeliefCategory,
        policy: RevisionPolicy,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            proposition: proposition.into(),
            confidence: confidence.clamp(BELIEF_CONFIDENCE_MIN, BELIEF_CONFIDENCE_MAX),
            category,
            policy,
            created_at: now,
            revised_at: now,
            revision_count: 0,
        }
    }

    /// Create a world belief with standard revision policy.
    pub fn world(proposition: impl Into<String>, confidence: f64) -> Self {
        Self::new(
            proposition,
            confidence,
            BeliefCategory::World,
            RevisionPolicy::Standard,
        )
    }

    /// Create a capability belief with protected revision policy.
    pub fn capability(proposition: impl Into<String>, confidence: f64) -> Self {
        Self::new(
            proposition,
            confidence,
            BeliefCategory::Capability,
            RevisionPolicy::Protected,
        )
    }

    /// Returns true if this belief's confidence can decrease.
    pub fn can_decrease(&self) -> bool {
        matches!(self.policy, RevisionPolicy::Standard)
    }

    /// Returns true if this belief can be revised at all.
    pub fn is_revisable(&self) -> bool {
        !matches!(self.policy, RevisionPolicy::Immutable)
    }

    /// Apply a delta to this belief's confidence.
    ///
    /// For Protected beliefs, negative deltas are ignored (confidence only goes up).
    /// For Immutable beliefs, this is a no-op.
    /// The result is always clamped to [0.0, 1.0].
    pub fn apply_delta(&mut self, delta: f64) {
        match self.policy {
            RevisionPolicy::Immutable => {}
            RevisionPolicy::Protected => {
                if delta > 0.0 {
                    self.confidence = (self.confidence + delta)
                        .clamp(BELIEF_CONFIDENCE_MIN, BELIEF_CONFIDENCE_MAX);
                    self.revised_at = Utc::now();
                    self.revision_count += 1;
                }
                // Negative delta silently ignored for Protected
            }
            RevisionPolicy::Standard => {
                self.confidence =
                    (self.confidence + delta).clamp(BELIEF_CONFIDENCE_MIN, BELIEF_CONFIDENCE_MAX);
                self.revised_at = Utc::now();
                self.revision_count += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_cannot_decrease() {
        let mut b = Belief::capability("I can code", 0.8);
        b.apply_delta(-0.5);
        assert!(
            (b.confidence - 0.8).abs() < f64::EPSILON,
            "protected belief must not decrease"
        );
        assert_eq!(b.revision_count, 0);
    }

    #[test]
    fn protected_can_increase() {
        let mut b = Belief::capability("I can code", 0.8);
        b.apply_delta(0.1);
        assert!((b.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(b.revision_count, 1);
    }

    #[test]
    fn standard_can_decrease() {
        let mut b = Belief::world("it will rain", 0.7);
        b.apply_delta(-0.3);
        assert!((b.confidence - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn immutable_unchanged() {
        let mut b = Belief::new(
            "axiom",
            1.0,
            BeliefCategory::World,
            RevisionPolicy::Immutable,
        );
        b.apply_delta(-0.5);
        assert!((b.confidence - 1.0).abs() < f64::EPSILON);
        assert_eq!(b.revision_count, 0);
    }

    #[test]
    fn confidence_clamped() {
        let mut b = Belief::world("test", 0.9);
        b.apply_delta(0.5);
        assert!((b.confidence - 1.0).abs() < f64::EPSILON);

        let mut b2 = Belief::world("test2", 0.1);
        b2.apply_delta(-0.5);
        assert!((b2.confidence - 0.0).abs() < f64::EPSILON);
    }
}
