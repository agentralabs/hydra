//! All constants for hydra-resurrection.
//! No magic numbers anywhere else in this crate.

/// Target time for warm restart (seconds).
/// If warm restart exceeds this, a warning is emitted.
pub const WARM_RESTART_TARGET_SECONDS: u64 = 10;

/// Number of delta checkpoints before a full checkpoint is forced.
pub const DELTAS_PER_FULL_CHECKPOINT: u64 = 100;

/// Maximum size of a single checkpoint file in bytes (50 MB).
pub const MAX_CHECKPOINT_SIZE_BYTES: usize = 50 * 1024 * 1024;

/// Directory name for checkpoint storage.
pub const CHECKPOINT_DIR: &str = "checkpoints";

/// File prefix for full checkpoint files.
pub const FULL_CHECKPOINT_PREFIX: &str = "full_";

/// File prefix for delta checkpoint files.
pub const DELTA_CHECKPOINT_PREFIX: &str = "delta_";

/// File name for the checkpoint index.
pub const INDEX_FILE_NAME: &str = "checkpoint_index.json";

/// Maximum number of checkpoints retained before pruning.
pub const MAX_CHECKPOINTS_RETAINED: usize = 50;

/// Timeout for a checkpoint write operation (milliseconds).
pub const CHECKPOINT_WRITE_TIMEOUT_MS: u64 = 1_000;
