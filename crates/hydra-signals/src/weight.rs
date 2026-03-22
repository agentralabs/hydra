//! Signal weight computation — wraps hydra-animus CCFT weight.
//!
//! Provides a fabric-level interface to the animus weight computation,
//! with sensible defaults for fabric-level signals.

use hydra_animus::semiring::weight::{compute_weight as animus_compute_weight, WeightInputs};
use hydra_animus::{Signal, SignalTier};

/// Compute the fabric-level weight for a signal.
///
/// Extracts the relevant inputs from the signal and delegates
/// to the animus CCFT weight computation.
pub fn compute_signal_weight(signal: &Signal) -> f64 {
    let mut inputs = WeightInputs::new(
        signal.source_trust_tier,
        signal.causal_chain.len(),
    );
    if is_constitutional(signal) {
        inputs = inputs.with_constitutional();
    }
    match animus_compute_weight(&inputs) {
        Ok(w) => w.value(),
        Err(_) => 0.0,
    }
}

/// Check whether a signal has constitutional relevance.
fn is_constitutional(signal: &Signal) -> bool {
    matches!(signal.tier, SignalTier::Constitution | SignalTier::Adversarial)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_animus::{
        graph::PrimeGraph,
        semiring::signal::{SignalId, SignalTier, SignalWeight},
    };

    fn make_signal(tier: SignalTier, trust: u8) -> Signal {
        Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            tier,
            trust,
        )
    }

    #[test]
    fn constitutional_signal_highest_weight() {
        let w = compute_signal_weight(&make_signal(SignalTier::Constitution, 0));
        assert!(
            w > 0.8,
            "constitutional signal weight should be high: {}",
            w
        );
    }

    #[test]
    fn fleet_signal_moderate_weight() {
        let w = compute_signal_weight(&make_signal(SignalTier::Fleet, 3));
        assert!(w > 0.0 && w < 1.0, "fleet signal weight: {}", w);
    }

    #[test]
    fn higher_trust_yields_higher_weight() {
        let high = compute_signal_weight(&make_signal(SignalTier::Fleet, 0));
        let low = compute_signal_weight(&make_signal(SignalTier::Fleet, 5));
        assert!(high > low);
    }
}
