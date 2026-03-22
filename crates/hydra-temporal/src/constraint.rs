//! Decision constraints — temporal rules that govern future actions.

use crate::decay::ConstraintDecay;
use crate::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

/// Unique identifier for a decision.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DecisionId(String);

impl DecisionId {
    /// Create a new `DecisionId` from a string value.
    pub fn from_value(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Return the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DecisionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The kind of constraint a decision imposes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintKind {
    /// Forbids a specific action pattern.
    Forbids,
    /// Requires a specific action before proceeding.
    Requires,
    /// Records a precedent — informational but strongly weighted.
    Precedent,
    /// Purely informational — no enforcement.
    Informational,
}

/// A single decision constraint in the temporal graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionConstraint {
    /// Unique identifier for this decision.
    pub id: DecisionId,
    /// When the decision was made.
    pub timestamp: Timestamp,
    /// What kind of constraint this is.
    pub kind: ConstraintKind,
    /// Human-readable description of the constraint.
    pub description: String,
    /// The action pattern this constraint applies to.
    pub action_pattern: String,
    /// Parent decision (if any) — forms the DAG edge.
    pub parent: Option<DecisionId>,
    /// Decay model for this constraint's strength.
    #[serde(skip)]
    pub decay: Option<ConstraintDecay>,
}

impl DecisionConstraint {
    /// Create a new decision constraint.
    pub fn new(
        id: DecisionId,
        timestamp: Timestamp,
        kind: ConstraintKind,
        description: String,
        action_pattern: String,
        parent: Option<DecisionId>,
        initial_strength: f64,
    ) -> Self {
        Self {
            id,
            timestamp,
            kind,
            description,
            action_pattern,
            parent,
            decay: Some(ConstraintDecay::new(initial_strength)),
        }
    }

    /// Check whether this constraint conflicts with a proposed action.
    ///
    /// Returns `Some(reason)` if the constraint is a `Forbids` kind and the
    /// proposed action matches the pattern. Otherwise returns `None`.
    pub fn check_conflict(&self, proposed_action: &str, elapsed_seconds: f64) -> Option<String> {
        if self.kind != ConstraintKind::Forbids {
            return None;
        }

        // Check if the proposed action contains the forbidden pattern
        if !proposed_action.contains(&self.action_pattern) {
            return None;
        }

        // Check if the constraint is still active
        let strength = self
            .decay
            .as_ref()
            .map(|d| d.strength_at(elapsed_seconds))
            .unwrap_or(1.0);

        if strength <= crate::constants::CONSTRAINT_DECAY_FLOOR {
            return None;
        }

        Some(format!(
            "Forbidden by decision '{}': {} (strength: {:.4})",
            self.id, self.description, strength
        ))
    }

    /// Get the current strength of this constraint.
    pub fn current_strength(&self, elapsed_seconds: f64) -> f64 {
        self.decay
            .as_ref()
            .map(|d| d.strength_at(elapsed_seconds))
            .unwrap_or(1.0)
    }
}
