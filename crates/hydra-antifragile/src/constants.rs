//! Constants for the antifragile crate.

/// Maximum number of resistance records.
pub const ANTIFRAGILE_MAX_RECORDS: usize = 100_000;

/// Initial resistance for a newly encountered obstacle class.
pub const INITIAL_RESISTANCE: f64 = 0.1;

/// Resistance increase per successful encounter.
pub const RESISTANCE_PER_WIN: f64 = 0.05;

/// Minimum resistance floor (resistance never drops below this after first encounter).
pub const RESISTANCE_FLOOR: f64 = 0.01;

/// Resistance decay per day since last encounter.
pub const DECAY_PER_DAY: f64 = 0.0001;
