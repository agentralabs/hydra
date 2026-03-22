//! SignalGate — entry-point validation for all signals entering the fabric.
//!
//! Every signal must pass the gate before routing. The gate checks:
//! 1. Orphan chain validation (via hydra-animus semiring)
//! 2. Chain depth limits
//! 3. Constitutional compliance (via hydra-constitution checker)

use crate::constants::GATE_MAX_CHAIN_DEPTH;
use crate::errors::SignalError;
use hydra_animus::semiring::orphan::validate_chain;
use hydra_animus::Signal;
use hydra_constitution::{ConstitutionChecker, LawCheckContext};

/// The signal entry gate. Validates every signal before it enters the fabric.
pub struct SignalGate {
    checker: ConstitutionChecker,
}

impl SignalGate {
    /// Create a new signal gate with a fresh constitution checker.
    pub fn new() -> Self {
        Self {
            checker: ConstitutionChecker::new(),
        }
    }

    /// Check whether a signal is permitted to enter the fabric.
    ///
    /// Returns `Ok(())` if the signal passes all checks.
    /// Returns `Err(SignalError)` describing the first failure.
    pub fn check(&self, signal: &Signal) -> Result<(), SignalError> {
        self.check_orphan(signal)?;
        self.check_depth(signal)?;
        self.check_constitutional(signal)?;
        Ok(())
    }

    /// Validate that the signal's causal chain is complete (not an orphan).
    fn check_orphan(&self, signal: &Signal) -> Result<(), SignalError> {
        validate_chain(signal).map_err(|_| SignalError::OrphanRejected {
            id: signal.id.as_str().to_string(),
        })
    }

    /// Validate that the signal's causal chain does not exceed the depth limit.
    fn check_depth(&self, signal: &Signal) -> Result<(), SignalError> {
        let depth = signal.causal_chain.len();
        if depth > GATE_MAX_CHAIN_DEPTH {
            return Err(SignalError::ChainTooDeep {
                id: signal.id.as_str().to_string(),
                depth,
                max: GATE_MAX_CHAIN_DEPTH,
            });
        }
        Ok(())
    }

    /// Validate that the signal does not violate any constitutional law.
    fn check_constitutional(&self, signal: &Signal) -> Result<(), SignalError> {
        let causal_chain: Vec<String> = signal
            .causal_chain
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();

        let ctx =
            LawCheckContext::new(signal.id.as_str(), "signal.emit").with_causal_chain(causal_chain);

        let result = self.checker.check(&ctx);
        if result.is_permitted() {
            Ok(())
        } else {
            let reason = result
                .first_violation()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown violation".to_string());
            Err(SignalError::ConstitutionalViolation {
                id: signal.id.as_str().to_string(),
                reason,
            })
        }
    }
}

impl Default for SignalGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_animus::{
        graph::PrimeGraph,
        semiring::signal::{SignalId, SignalTier, SignalWeight},
    };

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
    fn valid_signal_passes_gate() {
        let gate = SignalGate::new();
        let signal = valid_signal(SignalTier::Fleet);
        assert!(gate.check(&signal).is_ok());
    }

    #[test]
    fn orphan_signal_rejected() {
        let gate = SignalGate::new();
        let mut signal = valid_signal(SignalTier::Fleet);
        signal.causal_chain.clear();
        assert!(matches!(
            gate.check(&signal),
            Err(SignalError::OrphanRejected { .. })
        ));
    }

    #[test]
    fn constitutional_signal_passes_gate() {
        let gate = SignalGate::new();
        let signal = valid_signal(SignalTier::Constitution);
        assert!(gate.check(&signal).is_ok());
    }
}
