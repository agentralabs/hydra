/// How often (in days) a continuity checkpoint should be recorded.
pub const CHECKPOINT_INTERVAL_DAYS: u32 = 365;

/// Maximum number of checkpoints retained per entity arc.
pub const MAX_CHECKPOINTS: usize = 100;

/// Minimum checkpoints required for a lineage proof to be valid.
pub const MIN_CHECKPOINTS_FOR_PROOF: usize = 1;
