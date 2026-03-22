//! The Signal — the atomic unit of the Animus causal semiring.
//! Every inter-module message in Hydra is a Signal.

use crate::{
    constants::{
        SEMIRING_IDENTITY_ID, SEMIRING_ZERO_ID, SIGNAL_CHAIN_MAX_DEPTH, SIGNAL_WEIGHT_CEILING,
        SIGNAL_WEIGHT_FLOOR,
    },
    errors::AnimusError,
    graph::PrimeGraph,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a signal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignalId(String);

impl SignalId {
    /// Generate a new unique signal ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// The semiring multiplicative identity (constitutional root).
    pub fn identity() -> Self {
        Self(SEMIRING_IDENTITY_ID.to_string())
    }

    /// The semiring additive identity (null signal).
    pub fn zero() -> Self {
        Self(SEMIRING_ZERO_ID.to_string())
    }

    /// Returns the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true if this is the identity signal.
    pub fn is_identity(&self) -> bool {
        self.0 == SEMIRING_IDENTITY_ID
    }

    /// Returns true if this is the zero signal.
    pub fn is_zero(&self) -> bool {
        self.0 == SEMIRING_ZERO_ID
    }
}

impl Default for SignalId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SignalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The weight of a signal — how significant it is.
/// Computed from trust tier, causal depth, novelty, and constitutional relevance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SignalWeight(f64);

impl SignalWeight {
    /// Create a new weight, validated against [FLOOR, CEILING].
    pub fn new(value: f64) -> Result<Self, AnimusError> {
        if !(SIGNAL_WEIGHT_FLOOR..=SIGNAL_WEIGHT_CEILING).contains(&value) {
            return Err(AnimusError::InvalidSignalWeight {
                weight: value,
                min: SIGNAL_WEIGHT_FLOOR,
                max: SIGNAL_WEIGHT_CEILING,
            });
        }
        Ok(Self(value))
    }

    /// Returns the weight value.
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Maximum weight.
    pub fn max() -> Self {
        Self(SIGNAL_WEIGHT_CEILING)
    }
    /// Minimum weight.
    pub fn min() -> Self {
        Self(SIGNAL_WEIGHT_FLOOR)
    }
}

/// Which tier of Hydra's routing priority a signal falls into.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SignalTier {
    /// Constitutional signals — highest priority, bypass all queues.
    Constitution = 0,
    /// Adversarial/security signals — immediate routing.
    Adversarial = 1,
    /// Belief revision signals — high priority.
    BeliefRevision = 2,
    /// Fleet management signals — standard priority.
    Fleet = 3,
    /// Companion/personal signals — background.
    Companion = 4,
    /// Prediction and staging signals — ambient.
    Prediction = 5,
}

/// A signal in the Animus causal semiring.
/// The atomic unit of all inter-module communication in Hydra.
///
/// Mathematical type: element of (S, +, *, 0, 1)
/// Where:
///   - `*` = causal composition ("a caused b")
///   - `+` = signal merge ("a and b both contributed")
///   - `0` = null signal (SignalId::zero())
///   - `1` = constitutional identity (SignalId::identity())
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Unique ID of this signal.
    pub id: SignalId,

    /// The semantic content — a complete Prime graph.
    pub content: PrimeGraph,

    /// The ID of the signal that caused this one (parent).
    /// Must ultimately trace to SignalId::identity() via the chain.
    pub caused_by: SignalId,

    /// The full chain from this signal back to constitutional identity.
    /// chain[0] = caused_by, chain[last] = SignalId::identity().
    pub causal_chain: Vec<SignalId>,

    /// How significant this signal is.
    pub weight: SignalWeight,

    /// Which routing tier this signal belongs to.
    pub tier: SignalTier,

    /// The trust tier of the entity that emitted this signal.
    pub source_trust_tier: u8,

    /// When this signal was created.
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Ed25519 signature of (id + caused_by + content hash).
    /// Set by the bus after signing. Empty until signed.
    pub signature: Vec<u8>,
}

impl Signal {
    /// Create a new signal caused by the given parent.
    pub fn new(
        content: PrimeGraph,
        caused_by: SignalId,
        weight: SignalWeight,
        tier: SignalTier,
        source_trust_tier: u8,
    ) -> Self {
        let causal_chain = vec![caused_by.clone()];
        Self {
            id: SignalId::new(),
            content,
            caused_by,
            causal_chain,
            weight,
            tier,
            source_trust_tier,
            created_at: chrono::Utc::now(),
            signature: Vec::new(),
        }
    }

    /// Create the constitutional identity signal (semiring multiplicative identity).
    pub fn constitutional_identity() -> Self {
        Self {
            id: SignalId::identity(),
            content: PrimeGraph::new(),
            caused_by: SignalId::identity(),
            causal_chain: vec![SignalId::identity()],
            weight: SignalWeight::max(),
            tier: SignalTier::Constitution,
            source_trust_tier: 0,
            created_at: chrono::Utc::now(),
            signature: Vec::new(),
        }
    }

    /// True if this signal's chain terminates at the constitutional identity.
    pub fn chain_is_complete(&self) -> bool {
        self.causal_chain
            .last()
            .map(|id| id.is_identity())
            .unwrap_or(false)
    }

    /// True if this is an orphan signal (empty chain or incomplete).
    pub fn is_orphan(&self) -> bool {
        self.causal_chain.is_empty() || !self.chain_is_complete()
    }

    /// True if this is the constitutional identity signal.
    pub fn is_identity(&self) -> bool {
        self.id.is_identity()
    }

    /// The depth of the causal chain.
    pub fn chain_depth(&self) -> usize {
        self.causal_chain.len()
    }

    /// Validate the chain does not exceed maximum depth.
    pub fn validate_chain_depth(&self) -> Result<(), AnimusError> {
        if self.causal_chain.len() > SIGNAL_CHAIN_MAX_DEPTH {
            return Err(AnimusError::MalformedCausalChain {
                signal_id: self.id.to_string(),
                reason: format!(
                    "chain depth {} exceeds maximum {}",
                    self.causal_chain.len(),
                    SIGNAL_CHAIN_MAX_DEPTH
                ),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_signal() -> Signal {
        Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            SignalTier::Fleet,
            3,
        )
    }

    #[test]
    fn new_signal_chain_starts_with_caused_by() {
        let s = make_signal();
        assert_eq!(s.causal_chain[0], SignalId::identity());
    }

    #[test]
    fn chain_terminating_at_identity_is_complete() {
        let s = make_signal();
        assert!(s.chain_is_complete());
        assert!(!s.is_orphan());
    }

    #[test]
    fn empty_chain_is_orphan() {
        let mut s = make_signal();
        s.causal_chain.clear();
        assert!(s.is_orphan());
    }

    #[test]
    fn chain_not_ending_at_identity_is_orphan() {
        let mut s = make_signal();
        s.causal_chain = vec![SignalId::new()]; // random ID, not identity
        assert!(s.is_orphan());
    }

    #[test]
    fn identity_signal_is_identity() {
        let s = Signal::constitutional_identity();
        assert!(s.is_identity());
        assert!(s.chain_is_complete());
    }

    #[test]
    fn weight_floor_enforced() {
        assert!(SignalWeight::new(0.0).is_err());
        assert!(SignalWeight::new(0.001).is_ok());
    }

    #[test]
    fn weight_ceiling_enforced() {
        assert!(SignalWeight::new(1.0).is_ok());
        assert!(SignalWeight::new(1.001).is_err());
    }
}
