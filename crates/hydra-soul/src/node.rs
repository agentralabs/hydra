//! Meaning nodes — the atoms of the meaning graph.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::constants::{
    NODE_WEIGHT_DECAY_PER_DAY, NODE_WEIGHT_FLOOR, WEIGHT_PRESSURE_MULTIPLIER,
    WEIGHT_RETURN_MULTIPLIER,
};

/// The kind of meaning a node represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    /// A choice the principal keeps making.
    RecurringChoice,
    /// Something the principal keeps returning to.
    RecurringReturn,
    /// Something the principal consistently protects.
    RecurringProtection,
    /// Something the principal has never abandoned.
    NeverAbandoned,
    /// A commitment the principal keeps honoring.
    RecurringCommitment,
}

impl NodeKind {
    /// Returns the base weight multiplier for this node kind.
    ///
    /// Higher multipliers mean the node accumulates weight faster.
    pub fn base_multiplier(&self) -> f64 {
        match self {
            Self::RecurringChoice => 1.0,
            Self::RecurringReturn => WEIGHT_RETURN_MULTIPLIER,
            Self::RecurringProtection => WEIGHT_PRESSURE_MULTIPLIER,
            Self::NeverAbandoned => WEIGHT_PRESSURE_MULTIPLIER * WEIGHT_RETURN_MULTIPLIER,
            Self::RecurringCommitment => WEIGHT_RETURN_MULTIPLIER,
        }
    }
}

/// A single node in the meaning graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeaningNode {
    /// Human-readable label for what this node represents.
    pub label: String,
    /// The kind of meaning.
    pub kind: NodeKind,
    /// Current accumulated weight.
    pub weight: f64,
    /// Number of times this node has been reinforced.
    pub reinforcement_count: u64,
    /// When this node was first created.
    pub created_at: DateTime<Utc>,
    /// When this node was last reinforced.
    pub last_reinforced: DateTime<Utc>,
}

impl MeaningNode {
    /// Create a new meaning node with initial weight equal to its kind multiplier.
    pub fn new(label: impl Into<String>, kind: NodeKind) -> Self {
        let now = Utc::now();
        let weight = kind.base_multiplier();
        Self {
            label: label.into(),
            kind,
            weight,
            reinforcement_count: 1,
            created_at: now,
            last_reinforced: now,
        }
    }

    /// Reinforce this node, increasing its weight.
    pub fn reinforce(&mut self) {
        self.weight += self.kind.base_multiplier();
        self.reinforcement_count += 1;
        self.last_reinforced = Utc::now();
    }

    /// Apply exponential time-based decay to the node weight.
    ///
    /// w(t) = w₀ × e^(-λt) where λ = NODE_WEIGHT_DECAY_PER_DAY.
    /// Half-life = ln(2)/λ ≈ 6931 days ≈ 19 years.
    /// Exponential decay is smooth (no cliff) and physically correct.
    /// Weight never drops below the configured floor.
    pub fn decay(&mut self, days: f64) {
        self.weight = (self.weight * (-NODE_WEIGHT_DECAY_PER_DAY * days).exp())
            .max(NODE_WEIGHT_FLOOR);
    }

    /// Returns true if this node has decayed to the floor weight.
    pub fn is_fossil(&self) -> bool {
        (self.weight - NODE_WEIGHT_FLOOR).abs() < f64::EPSILON
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_weight_on_creation() {
        let node = MeaningNode::new("test", NodeKind::RecurringChoice);
        assert!(node.weight > 0.0, "new node must have positive weight");
    }

    #[test]
    fn reinforcement_increases_weight() {
        let mut node = MeaningNode::new("test", NodeKind::RecurringChoice);
        let before = node.weight;
        node.reinforce();
        assert!(node.weight > before, "reinforcement must increase weight");
    }

    #[test]
    fn decay_never_below_floor() {
        let mut node = MeaningNode::new("test", NodeKind::RecurringChoice);
        node.decay(1_000_000.0);
        assert!(
            node.weight >= NODE_WEIGHT_FLOOR,
            "weight must never drop below floor"
        );
    }

    #[test]
    fn protection_heavier_than_choice() {
        let choice = MeaningNode::new("c", NodeKind::RecurringChoice);
        let protection = MeaningNode::new("p", NodeKind::RecurringProtection);
        assert!(
            protection.weight > choice.weight,
            "protection must weigh more than choice"
        );
    }

    #[test]
    fn never_abandoned_heaviest() {
        let choice = MeaningNode::new("c", NodeKind::RecurringChoice);
        let ret = MeaningNode::new("r", NodeKind::RecurringReturn);
        let prot = MeaningNode::new("p", NodeKind::RecurringProtection);
        let abandoned = MeaningNode::new("a", NodeKind::NeverAbandoned);
        assert!(abandoned.weight > choice.weight);
        assert!(abandoned.weight > ret.weight);
        assert!(abandoned.weight > prot.weight);
    }
}
