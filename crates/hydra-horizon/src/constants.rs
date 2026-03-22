//! All constants for hydra-horizon.
//! No magic numbers or strings anywhere else in this crate.

/// Factor by which genome entry count influences perception expansion.
pub const PERCEPTION_GENOME_FACTOR: f64 = 0.0001;

/// Factor by which genome entry count influences action expansion.
pub const ACTION_SYNTHESIS_FACTOR: f64 = 0.0002;

/// Initial value for perception horizon.
pub const PERCEPTION_HORIZON_INITIAL: f64 = 0.1;

/// Initial value for action horizon.
pub const ACTION_HORIZON_INITIAL: f64 = 0.1;

/// Maximum horizon value (hard cap).
pub const HORIZON_MAX: f64 = 1.0;

/// Minimum delta for horizon expansion (no contraction).
pub const HORIZON_MIN_DELTA: f64 = 0.0;
