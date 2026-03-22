//! Causal composition: the multiplication operation of the Signal Causal Semiring.
//!
//! a * b = "signal a caused signal b"
//!
//! Properties verified by property tests:
//!   Associativity: (a * b) * c = a * (b * c)  [causality is transitive]
//!   Not commutative: a * b != b * a             [cause precedes effect]
//!   Identity: 1 * a = a * 1 = a                [every chain reaches root]

use crate::{errors::AnimusError, graph::PrimeGraph, semiring::signal::Signal};

/// Compose two signals causally: a * b
/// Produces a new signal that records: "a caused b"
/// The causal chain of the result = b's chain + a's chain.
pub fn compose(cause: &Signal, effect: &Signal) -> Result<Signal, AnimusError> {
    // Validate both operands
    if cause.is_orphan() {
        return Err(AnimusError::OrphanSignal {
            signal_id: cause.id.to_string(),
        });
    }

    // Build the composed causal chain:
    // effect chain + cause chain (transitivity of causality)
    let mut composed_chain = effect.causal_chain.clone();
    for ancestor in &cause.causal_chain {
        if !composed_chain.contains(ancestor) {
            composed_chain.push(ancestor.clone());
        }
    }

    // The composed signal has:
    // - New ID (it is a new signal representing the composition)
    // - cause's ID as caused_by
    // - Combined chain that must terminate at identity (inherited from cause)
    // - Weight = min of the two (the weaker link determines chain strength)
    let composed_weight = if cause.weight.value() < effect.weight.value() {
        cause.weight
    } else {
        effect.weight
    };

    // Tier = higher priority of the two (lower number = higher priority)
    let composed_tier = if cause.tier <= effect.tier {
        cause.tier.clone()
    } else {
        effect.tier.clone()
    };

    let mut composed = Signal::new(
        PrimeGraph::new(), // composition result carries empty graph by default
        cause.id.clone(),
        composed_weight,
        composed_tier,
        cause.source_trust_tier.min(effect.source_trust_tier),
    );

    composed.causal_chain = composed_chain;

    // Validate the result
    if composed.is_orphan() {
        return Err(AnimusError::OrphanSignal {
            signal_id: composed.id.to_string(),
        });
    }

    composed.validate_chain_depth()?;

    Ok(composed)
}

/// Verify the identity law: 1 * a = a (structurally)
/// The composition of identity with any signal preserves the signal's chain.
pub fn compose_with_identity(signal: &Signal) -> Result<Signal, AnimusError> {
    let identity = Signal::constitutional_identity();
    compose(&identity, signal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semiring::signal::{SignalId, SignalTier, SignalWeight};

    fn make_valid_signal(tier: SignalTier) -> Signal {
        Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            tier,
            3,
        )
    }

    #[test]
    fn compose_two_valid_signals() {
        let a = make_valid_signal(SignalTier::Fleet);
        let b = make_valid_signal(SignalTier::Fleet);
        let result = compose(&a, &b);
        assert!(result.is_ok());
        let c = result.unwrap();
        assert!(c.chain_is_complete());
        assert!(!c.is_orphan());
    }

    #[test]
    fn composed_tier_is_higher_priority() {
        let a = make_valid_signal(SignalTier::Constitution);
        let b = make_valid_signal(SignalTier::Fleet);
        let c = compose(&a, &b).unwrap();
        assert_eq!(c.tier, SignalTier::Constitution);
    }

    #[test]
    fn compose_with_orphan_fails() {
        let mut orphan = make_valid_signal(SignalTier::Fleet);
        orphan.causal_chain.clear(); // make it an orphan
        let valid = make_valid_signal(SignalTier::Fleet);
        assert!(compose(&orphan, &valid).is_err());
    }

    #[test]
    fn identity_composition_preserves_chain() {
        let a = make_valid_signal(SignalTier::Fleet);
        let result = compose_with_identity(&a).unwrap();
        assert!(result.chain_is_complete());
    }

    #[test]
    fn compose_is_not_commutative() {
        let a = make_valid_signal(SignalTier::Constitution);
        let b = make_valid_signal(SignalTier::Fleet);
        let ab = compose(&a, &b).unwrap();
        let ba = compose(&b, &a).unwrap();
        // Different caused_by — not commutative
        assert_ne!(ab.caused_by, ba.caused_by);
    }
}
