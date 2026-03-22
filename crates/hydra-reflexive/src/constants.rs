//! All constants for hydra-reflexive.
//! No magic numbers or strings anywhere else in this crate.

/// Maximum number of capabilities the self-model can track.
pub const MAX_CAPABILITIES: usize = 1024;

/// Maximum number of modification history entries retained.
pub const MAX_MODIFICATION_HISTORY: usize = 10_000;

/// How many ticks between automatic self-model updates.
pub const SELF_MODEL_UPDATE_INTERVAL_TICKS: u64 = 10;

/// How many snapshots the rollback window retains.
pub const ROLLBACK_WINDOW: usize = 50;

/// Timeout in milliseconds for a modification constitutional check.
pub const MODIFICATION_CHECK_TIMEOUT_MS: u64 = 100;
