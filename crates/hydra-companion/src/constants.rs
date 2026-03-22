//! All constants for hydra-companion.
//! No magic numbers anywhere else in this crate.

/// Maximum number of signals retained in the buffer.
pub const MAX_SIGNAL_BUFFER: usize = 100;

/// Maximum number of concurrent tasks.
pub const MAX_CONCURRENT_TASKS: usize = 8;

/// Noise threshold — signals with relevance below this are classified as Noise.
pub const NOISE_THRESHOLD: f64 = 0.2;

/// Default signal relevance when no classifier matches.
pub const DEFAULT_SIGNAL_RELEVANCE: f64 = 0.5;

/// How often the companion checks for new signals (ms).
pub const SIGNAL_POLL_INTERVAL_MS: u64 = 50;

/// Task timeout — cancel tasks that exceed this duration (seconds).
pub const TASK_TIMEOUT_SECONDS: u64 = 300;
