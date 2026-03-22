//! Orphan signal detection and causal chain validation.
//! Enforces Constitutional Law 7 at the semiring level.

use crate::{
    constants::{SEMIRING_IDENTITY_ID, SIGNAL_CHAIN_MAX_DEPTH},
    errors::AnimusError,
    semiring::signal::{Signal, SignalId},
};

/// Validates that a signal's causal chain is complete and well-formed.
pub fn validate_chain(signal: &Signal) -> Result<(), AnimusError> {
    check_non_empty(signal)?;
    check_terminates_at_identity(signal)?;
    check_depth(signal)?;
    check_no_empty_entries(signal)?;
    Ok(())
}

fn check_non_empty(signal: &Signal) -> Result<(), AnimusError> {
    if signal.causal_chain.is_empty() {
        return Err(AnimusError::OrphanSignal {
            signal_id: signal.id.to_string(),
        });
    }
    Ok(())
}

fn check_terminates_at_identity(signal: &Signal) -> Result<(), AnimusError> {
    let terminates = signal
        .causal_chain
        .last()
        .map(|id| id.as_str() == SEMIRING_IDENTITY_ID)
        .unwrap_or(false);

    if !terminates {
        return Err(AnimusError::MalformedCausalChain {
            signal_id: signal.id.to_string(),
            reason: format!(
                "chain terminates at '{}' not at constitutional identity",
                signal
                    .causal_chain
                    .last()
                    .map(|id| id.as_str())
                    .unwrap_or("(empty)")
            ),
        });
    }
    Ok(())
}

fn check_depth(signal: &Signal) -> Result<(), AnimusError> {
    if signal.causal_chain.len() > SIGNAL_CHAIN_MAX_DEPTH {
        return Err(AnimusError::MalformedCausalChain {
            signal_id: signal.id.to_string(),
            reason: format!(
                "depth {} exceeds maximum {}",
                signal.causal_chain.len(),
                SIGNAL_CHAIN_MAX_DEPTH
            ),
        });
    }
    Ok(())
}

fn check_no_empty_entries(signal: &Signal) -> Result<(), AnimusError> {
    for (i, entry) in signal.causal_chain.iter().enumerate() {
        if entry.as_str().is_empty() {
            return Err(AnimusError::MalformedCausalChain {
                signal_id: signal.id.to_string(),
                reason: format!("entry at index {} is empty", i),
            });
        }
    }
    Ok(())
}

/// Returns true if the signal is an orphan (fails chain validation).
pub fn is_orphan(signal: &Signal) -> bool {
    validate_chain(signal).is_err()
}

/// Build a valid minimal causal chain: [signal_id, constitutional_identity].
pub fn build_minimal_chain(signal_id: &SignalId) -> Vec<SignalId> {
    vec![signal_id.clone(), SignalId::identity()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        graph::PrimeGraph,
        semiring::signal::{SignalTier, SignalWeight},
    };

    fn valid_signal() -> Signal {
        Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            SignalTier::Fleet,
            3,
        )
    }

    #[test]
    fn valid_signal_passes_validation() {
        assert!(validate_chain(&valid_signal()).is_ok());
    }

    #[test]
    fn empty_chain_is_orphan() {
        let mut s = valid_signal();
        s.causal_chain.clear();
        assert!(validate_chain(&s).is_err());
        assert!(is_orphan(&s));
    }

    #[test]
    fn chain_not_at_identity_is_orphan() {
        let mut s = valid_signal();
        s.causal_chain = vec![SignalId::new()]; // random, not identity
        assert!(is_orphan(&s));
    }

    #[test]
    fn minimal_chain_is_valid() {
        let id = SignalId::new();
        let chain = build_minimal_chain(&id);
        assert_eq!(chain.len(), 2);
        assert!(chain.last().unwrap().is_identity());
    }

    #[test]
    fn identity_signal_passes_validation() {
        assert!(validate_chain(&Signal::constitutional_identity()).is_ok());
    }
}
