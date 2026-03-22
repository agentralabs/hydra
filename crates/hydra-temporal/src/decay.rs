//! Exponential decay model for constraint strength.
//!
//! S(t) = S0 * e^(-lambda * t), floored at `CONSTRAINT_DECAY_FLOOR`.

use crate::constants::{CONSTRAINT_DECAY_FLOOR, CONSTRAINT_DECAY_LAMBDA};

/// Manages exponential decay of constraint strength over time.
#[derive(Debug, Clone)]
pub struct ConstraintDecay {
    /// Initial strength (S0). Must be in (0.0, 1.0].
    initial_strength: f64,
    /// Decay rate (lambda). Defaults to `CONSTRAINT_DECAY_LAMBDA`.
    lambda: f64,
    /// Minimum strength floor.
    floor: f64,
}

impl ConstraintDecay {
    /// Create a new decay model with default lambda and floor.
    pub fn new(initial_strength: f64) -> Self {
        Self {
            initial_strength: initial_strength.clamp(0.0, 1.0),
            lambda: CONSTRAINT_DECAY_LAMBDA,
            floor: CONSTRAINT_DECAY_FLOOR,
        }
    }

    /// Create a decay model with custom parameters.
    pub fn with_params(initial_strength: f64, lambda: f64, floor: f64) -> Self {
        Self {
            initial_strength: initial_strength.clamp(0.0, 1.0),
            lambda,
            floor: floor.max(0.0),
        }
    }

    /// Compute strength at time `elapsed_seconds` after creation.
    ///
    /// Returns a value in [`floor`, `initial_strength`].
    pub fn strength_at(&self, elapsed_seconds: f64) -> f64 {
        let raw = self.initial_strength * (-self.lambda * elapsed_seconds).exp();
        raw.max(self.floor)
    }

    /// Whether the constraint has decayed to (or below) the floor.
    pub fn is_fossil(&self, elapsed_seconds: f64) -> bool {
        let raw = self.initial_strength * (-self.lambda * elapsed_seconds).exp();
        raw <= self.floor
    }

    /// Whether the constraint is still meaningfully active (above floor).
    pub fn is_active(&self, elapsed_seconds: f64) -> bool {
        !self.is_fossil(elapsed_seconds)
    }

    /// Compute seconds until the constraint reaches the fossil floor.
    ///
    /// Returns `None` if `initial_strength <= floor` (already fossil).
    pub fn time_to_fossil_seconds(&self) -> Option<f64> {
        if self.initial_strength <= self.floor {
            return None;
        }
        // S0 * e^(-lambda * t) = floor
        // t = -ln(floor / S0) / lambda
        let t = -(self.floor / self.initial_strength).ln() / self.lambda;
        Some(t)
    }

    /// Return the initial strength.
    pub fn initial_strength(&self) -> f64 {
        self.initial_strength
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strength_at_zero_is_initial() {
        let decay = ConstraintDecay::new(1.0);
        assert!((decay.strength_at(0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn strength_decays_over_time() {
        let decay = ConstraintDecay::new(1.0);
        let s1 = decay.strength_at(1_000_000.0);
        let s2 = decay.strength_at(10_000_000.0);
        assert!(s1 > s2);
    }

    #[test]
    fn floor_is_enforced() {
        let decay = ConstraintDecay::new(1.0);
        let s = decay.strength_at(1e15);
        assert!((s - CONSTRAINT_DECAY_FLOOR).abs() < 1e-10);
    }

    #[test]
    fn fossil_detection() {
        let decay = ConstraintDecay::new(1.0);
        assert!(!decay.is_fossil(0.0));
        assert!(decay.is_fossil(1e15));
    }

    #[test]
    fn time_to_fossil_exists() {
        let decay = ConstraintDecay::new(1.0);
        let t = decay.time_to_fossil_seconds().unwrap();
        assert!(t > 0.0);
        // Slightly past the boundary, should definitely be fossil
        assert!(decay.is_fossil(t + 1.0));
        // Strength at exact boundary should be very close to floor
        let s = decay.strength_at(t);
        assert!((s - CONSTRAINT_DECAY_FLOOR).abs() < 1e-6);
    }
}
