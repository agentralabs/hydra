//! All constants for hydra-morphic.
//! No magic numbers or strings anywhere else in this crate.

/// Current version of the morphic signature format.
pub const MORPHIC_VERSION: u32 = 1;

/// Maximum number of events in the morphic history.
pub const MORPHIC_HISTORY_MAX: usize = 100_000;

/// Distance threshold: two identities with distance > this are different entities.
pub const IDENTITY_DISTANCE_THRESHOLD: f64 = 0.3;

/// Weight of capability history in morphic distance computation.
pub const MORPHIC_CAPABILITY_WEIGHT: f64 = 0.4;

/// Weight of modification history in morphic distance.
pub const MORPHIC_MODIFICATION_WEIGHT: f64 = 0.3;

/// Weight of memory continuity in morphic distance.
pub const MORPHIC_MEMORY_WEIGHT: f64 = 0.3;
