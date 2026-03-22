//! Signal weight computation using CCFT coefficients.
//! Implements: w(s) = α·trust_tier + β·causal_depth + γ·novelty + δ·constitutional
//!
//! Coefficients match HYDRA-MASTER-SPEC-V3.md Section 3:
//!   α (trust tier contribution)      = 0.40
//!   β (causal depth contribution)    = 0.25
//!   γ (novelty contribution)         = 0.20
//!   δ (constitutional relevance)     = 0.15

use crate::{
    constants::{
        GROWTH_SIGNAL_MIN_BOOST, SIGNAL_WEIGHT_ALPHA, SIGNAL_WEIGHT_BETA, SIGNAL_WEIGHT_CEILING,
        SIGNAL_WEIGHT_DELTA, SIGNAL_WEIGHT_FLOOR, SIGNAL_WEIGHT_GAMMA, TRUST_TIER_COUNT,
    },
    errors::AnimusError,
    semiring::signal::{SignalTier, SignalWeight},
};

/// Inputs for the CCFT signal weight computation.
#[derive(Debug, Clone)]
pub struct WeightInputs {
    /// Trust tier of the signal source (0 = constitution, 5 = external).
    pub source_trust_tier: u8,

    /// Depth of the causal chain.
    pub causal_chain_depth: usize,

    /// Whether this signal carries novel information not seen before.
    pub is_novel: bool,

    /// Whether this signal has constitutional relevance (touches a law).
    pub is_constitutional: bool,

    /// The routing tier of the signal.
    pub tier: SignalTier,
}

impl WeightInputs {
    /// Create new weight inputs with defaults (not novel, not constitutional, Fleet tier).
    pub fn new(source_trust_tier: u8, causal_chain_depth: usize) -> Self {
        Self {
            source_trust_tier,
            causal_chain_depth,
            is_novel: false,
            is_constitutional: false,
            tier: SignalTier::Fleet,
        }
    }

    /// Mark signal as carrying novel information.
    pub fn with_novel(mut self) -> Self {
        self.is_novel = true;
        self
    }

    /// Mark signal as constitutionally relevant.
    pub fn with_constitutional(mut self) -> Self {
        self.is_constitutional = true;
        self
    }

    /// Set the signal routing tier.
    pub fn with_tier(mut self, tier: SignalTier) -> Self {
        self.tier = tier;
        self
    }
}

/// Compute signal weight using the CCFT formula.
///
/// w(s) = α·trust_factor + β·depth_factor + γ·novelty_factor + δ·constitutional_factor
///
/// All factors are normalized to [0, 1] before weighting.
/// Growth layer signals (BeliefRevision tier) receive a minimum boost.
/// Result is clamped to [SIGNAL_WEIGHT_FLOOR, SIGNAL_WEIGHT_CEILING].
pub fn compute_weight(inputs: &WeightInputs) -> Result<SignalWeight, AnimusError> {
    // Normalize trust tier: tier 0 (constitution) = 1.0, tier 5 (external) = 0.0
    let trust_factor = if inputs.source_trust_tier == 0 {
        1.0_f64
    } else {
        let max_tier = (TRUST_TIER_COUNT - 1) as f64;
        1.0 - (inputs.source_trust_tier as f64 / max_tier)
    };

    // Normalize causal depth: deeper chains = higher weight (more context)
    // Use logarithmic scaling: depth 1 = low, depth 10+ = near 1.0
    // Guard: depth=0 treated as depth=1 to avoid ln(0) = -infinity → NaN
    let depth_factor = {
        let depth = inputs.causal_chain_depth.max(1) as f64;
        (depth.ln() + 1.0) / (depth.ln() + 2.0)
    };

    // Novelty: binary for now (novel = 1.0, not novel = 0.2)
    let novelty_factor = if inputs.is_novel { 1.0 } else { 0.2 };

    // Constitutional relevance: binary
    let constitutional_factor = if inputs.is_constitutional { 1.0 } else { 0.0 };

    // CCFT weight formula
    let raw_weight = SIGNAL_WEIGHT_ALPHA * trust_factor
        + SIGNAL_WEIGHT_BETA * depth_factor
        + SIGNAL_WEIGHT_GAMMA * novelty_factor
        + SIGNAL_WEIGHT_DELTA * constitutional_factor;

    // Constitutional signals get a minimum weight — the constitution
    // is the highest authority, never weakened by shallow chains or low novelty.
    // Growth layer signals get a separate minimum boost.
    let boosted = if inputs.is_constitutional {
        raw_weight.max(0.85)
    } else if is_growth_signal(&inputs.tier) {
        raw_weight.max(GROWTH_SIGNAL_MIN_BOOST)
    } else {
        raw_weight
    };

    // Clamp to valid range
    let clamped = boosted.clamp(SIGNAL_WEIGHT_FLOOR, SIGNAL_WEIGHT_CEILING);

    SignalWeight::new(clamped)
}

/// Returns true if the signal tier indicates a growth layer origin.
/// Growth layer signals are always given minimum weight to ensure
/// capability-related signals are never dropped as noise.
fn is_growth_signal(tier: &SignalTier) -> bool {
    // Growth layer signals route through BeliefRevision tier
    // (they update the belief manifold when capabilities change)
    matches!(tier, SignalTier::BeliefRevision)
}

/// Verify that the four CCFT coefficients sum to 1.0 (within epsilon).
pub fn verify_coefficient_sum() -> bool {
    let sum = SIGNAL_WEIGHT_ALPHA + SIGNAL_WEIGHT_BETA + SIGNAL_WEIGHT_GAMMA + SIGNAL_WEIGHT_DELTA;
    (sum - 1.0).abs() < 1e-10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coefficients_sum_to_one() {
        assert!(
            verify_coefficient_sum(),
            "CCFT weight coefficients must sum to 1.0"
        );
    }

    #[test]
    fn constitution_source_gets_high_weight() {
        let inputs = WeightInputs::new(0, 3).with_constitutional();
        let weight = compute_weight(&inputs).expect("weight computation failed");
        assert!(
            weight.value() > 0.7,
            "constitutional source with constitutional relevance should be high, got {}",
            weight.value()
        );
    }

    #[test]
    fn external_source_gets_lower_weight() {
        let inputs = WeightInputs::new(5, 1);
        let weight = compute_weight(&inputs).expect("weight computation failed");
        let inputs_internal = WeightInputs::new(1, 1);
        let weight_internal = compute_weight(&inputs_internal).expect("weight computation failed");
        assert!(weight_internal.value() > weight.value());
    }

    #[test]
    fn novel_signal_gets_higher_weight() {
        let base = WeightInputs::new(3, 2);
        let novel = WeightInputs::new(3, 2).with_novel();
        let w_base = compute_weight(&base).expect("weight computation failed");
        let w_novel = compute_weight(&novel).expect("weight computation failed");
        assert!(w_novel.value() > w_base.value());
    }

    #[test]
    fn weight_always_in_valid_range() {
        let test_cases = vec![
            WeightInputs::new(0, 1),
            WeightInputs::new(5, 100),
            WeightInputs::new(3, 1).with_novel().with_constitutional(),
            WeightInputs::new(0, 1).with_tier(SignalTier::BeliefRevision),
        ];
        for inputs in test_cases {
            let w = compute_weight(&inputs).expect("weight computation failed");
            assert!(w.value() >= SIGNAL_WEIGHT_FLOOR);
            assert!(w.value() <= SIGNAL_WEIGHT_CEILING);
        }
    }

    #[test]
    fn growth_signals_get_minimum_boost() {
        let inputs = WeightInputs::new(5, 1).with_tier(SignalTier::BeliefRevision);
        let weight = compute_weight(&inputs).expect("weight computation failed");
        assert!(
            weight.value() >= GROWTH_SIGNAL_MIN_BOOST,
            "growth signals must be boosted to min {}, got {}",
            GROWTH_SIGNAL_MIN_BOOST,
            weight.value()
        );
    }

    #[test]
    fn deeper_chain_increases_weight() {
        let shallow = WeightInputs::new(3, 1);
        let deep = WeightInputs::new(3, 50);
        let w_shallow = compute_weight(&shallow).expect("weight computation failed");
        let w_deep = compute_weight(&deep).expect("weight computation failed");
        assert!(w_deep.value() > w_shallow.value());
    }
}
