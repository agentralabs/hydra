//! Spawn decisions based on Boltzmann weights.
//!
//! The trust thermodynamic model determines which tiers are permitted
//! to spawn new agents based on their energy levels and temperature.

use crate::constants::*;
use crate::score::TrustTier;

/// Compute the Boltzmann weight for a given tier at a given temperature.
///
/// Weight = exp(-E / (k * T)) where E is tier energy, k is Boltzmann constant,
/// and T is temperature.
pub fn boltzmann_weight(tier: TrustTier, temperature: f64) -> f64 {
    let energy = tier.energy();
    if temperature <= 0.0 {
        if energy == 0.0 {
            return 1.0;
        }
        return 0.0;
    }
    (-energy / (BOLTZMANN_K * temperature)).exp()
}

/// Decide whether an agent at the given tier should be permitted to spawn.
///
/// Constitution tier is ALWAYS permitted (returns true regardless of weight).
/// Other tiers must have a Boltzmann weight above 0.5.
pub fn spawn_decision(tier: TrustTier, temperature: f64) -> bool {
    // Constitution is always permitted to spawn.
    if tier == TrustTier::Platinum {
        return true;
    }
    boltzmann_weight(tier, temperature) > 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platinum_always_spawns() {
        assert!(spawn_decision(TrustTier::Platinum, 0.001));
        assert!(spawn_decision(TrustTier::Platinum, 100.0));
    }

    #[test]
    fn boltzmann_ordering() {
        let t = DEFAULT_TEMPERATURE;
        let w_plat = boltzmann_weight(TrustTier::Platinum, t);
        let w_gold = boltzmann_weight(TrustTier::Gold, t);
        let w_silver = boltzmann_weight(TrustTier::Silver, t);
        let w_bronze = boltzmann_weight(TrustTier::Bronze, t);
        assert!(w_plat > w_gold);
        assert!(w_gold > w_silver);
        assert!(w_silver > w_bronze);
    }
}
