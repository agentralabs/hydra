//! All constants for hydra-adversary.
//! No magic numbers or strings anywhere else in this crate.

/// Threshold for antibody recognition (cosine similarity).
pub const ANTIBODY_RECOGNITION_THRESHOLD: f64 = 0.7;

/// Confidence boost when an antibody successfully blocks a threat.
pub const ANTIBODY_CONFIDENCE_BOOST: f64 = 0.05;

/// Decay rate for antifragile resistance (per time unit).
pub const ANTIFRAGILE_DECAY_RATE: f64 = 0.01;

/// Minimum floor for antifragile resistance (never drops below).
pub const ANTIFRAGILE_FLOOR: f64 = 0.1;

/// Maximum number of antibodies in the immune system.
pub const MAX_ANTIBODIES: usize = 4096;

/// Maximum number of threat actors in the ecology.
pub const MAX_THREAT_ACTORS: usize = 256;

/// Maximum number of antifragile records.
pub const MAX_ANTIFRAGILE_RECORDS: usize = 2048;

/// Resistance gained per successful defense.
pub const RESISTANCE_PER_WIN: f64 = 0.05;

/// Initial resistance for new antifragile records.
pub const INITIAL_RESISTANCE: f64 = 0.1;

/// Maximum resistance value.
pub const MAX_RESISTANCE: f64 = 1.0;
