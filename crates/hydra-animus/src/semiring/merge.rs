//! Signal merge: the addition operation of the Signal Causal Semiring.
//!
//! a + b = "signals a and b both contributed to a combined signal"
//!
//! Properties:
//!   Commutative: a + b = b + a     (order of contributions doesn't matter)
//!   Associative: (a + b) + c = a + (b + c)
//!   Identity:    0 + a = a          (merging with null signal = identity)

use crate::{errors::AnimusError, graph::PrimeGraph, semiring::signal::Signal};

/// Merge two signals: a + b
/// Both signals contributed to a shared outcome.
/// The result's chain contains all ancestors from both.
pub fn merge(a: &Signal, b: &Signal) -> Result<Signal, AnimusError> {
    if a.is_orphan() {
        return Err(AnimusError::OrphanSignal {
            signal_id: a.id.to_string(),
        });
    }

    if b.is_orphan() {
        return Err(AnimusError::OrphanSignal {
            signal_id: b.id.to_string(),
        });
    }

    // Merged chain: union of both chains, deduped, preserving order
    let mut merged_chain = a.causal_chain.clone();
    for ancestor in &b.causal_chain {
        if !merged_chain.contains(ancestor) {
            merged_chain.push(ancestor.clone());
        }
    }

    // Weight = max of the two (merge amplifies, not weakens)
    let merged_weight = if a.weight.value() >= b.weight.value() {
        a.weight
    } else {
        b.weight
    };

    // Tier = higher priority of the two
    let merged_tier = if a.tier <= b.tier {
        a.tier.clone()
    } else {
        b.tier.clone()
    };

    // Trust tier = lower number (higher authority) of the two sources
    let merged_trust = a.source_trust_tier.min(b.source_trust_tier);

    let mut merged = Signal::new(
        PrimeGraph::new(),
        a.id.clone(), // primary contributor is 'a'
        merged_weight,
        merged_tier,
        merged_trust,
    );

    merged.causal_chain = merged_chain;

    if merged.is_orphan() {
        return Err(AnimusError::OrphanSignal {
            signal_id: merged.id.to_string(),
        });
    }

    merged.validate_chain_depth()?;

    Ok(merged)
}

/// Identity law: 0 + a = a (structurally)
/// Merging with the null signal returns a structurally equivalent signal.
pub fn merge_with_zero(signal: &Signal) -> Result<Signal, AnimusError> {
    // 0 is the additive identity — merging with it is a no-op structurally
    // We return a new signal with the same chain
    if signal.is_orphan() {
        return Err(AnimusError::OrphanSignal {
            signal_id: signal.id.to_string(),
        });
    }

    let mut result = Signal::new(
        PrimeGraph::new(),
        signal.caused_by.clone(),
        signal.weight,
        signal.tier.clone(),
        signal.source_trust_tier,
    );
    result.causal_chain = signal.causal_chain.clone();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semiring::signal::{SignalId, SignalTier, SignalWeight};

    fn valid_signal(tier: SignalTier) -> Signal {
        Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            tier,
            3,
        )
    }

    #[test]
    fn merge_two_valid_signals() {
        let a = valid_signal(SignalTier::Fleet);
        let b = valid_signal(SignalTier::Companion);
        let c = merge(&a, &b).unwrap();
        assert!(c.chain_is_complete());
        assert!(!c.is_orphan());
    }

    #[test]
    fn merged_tier_is_higher_priority() {
        let a = valid_signal(SignalTier::BeliefRevision);
        let b = valid_signal(SignalTier::Companion);
        let c = merge(&a, &b).unwrap();
        assert_eq!(c.tier, SignalTier::BeliefRevision);
    }

    #[test]
    fn merge_with_orphan_fails() {
        let mut orphan = valid_signal(SignalTier::Fleet);
        orphan.causal_chain.clear();
        let valid = valid_signal(SignalTier::Fleet);
        assert!(merge(&orphan, &valid).is_err());
        assert!(merge(&valid, &orphan).is_err());
    }

    #[test]
    fn merge_with_zero_preserves_chain() {
        let a = valid_signal(SignalTier::Fleet);
        let result = merge_with_zero(&a).unwrap();
        assert!(result.chain_is_complete());
    }
}
