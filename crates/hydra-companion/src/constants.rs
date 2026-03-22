//! All constants for hydra-companion.
//! No magic numbers anywhere else in this crate.
//! Values aligned with PHASE-13-CLAUDE-CODE-INSTRUCTIONS.md spec.

/// Maximum number of signals retained in the buffer (spec: 1,000).
pub const MAX_SIGNAL_BUFFER: usize = 1_000;

/// Maximum number of concurrent tasks (spec: 64).
pub const MAX_CONCURRENT_TASKS: usize = 64;

/// Maximum digest items returned per /digest call.
pub const MAX_DIGEST_ITEMS: usize = 100;

/// Batch window — signals older than this (seconds) are auto-batched.
pub const SIGNAL_BATCH_WINDOW_SECONDS: u64 = 3_600;

/// Noise threshold — signals with relevance below this are classified as Noise.
pub const NOISE_THRESHOLD: f64 = 0.2;

/// Default signal relevance when no classifier matches.
pub const DEFAULT_SIGNAL_RELEVANCE: f64 = 0.5;

/// How often the companion checks for new signals (ms).
pub const SIGNAL_POLL_INTERVAL_MS: u64 = 50;

/// Task timeout — cancel tasks that exceed this duration (seconds).
pub const TASK_TIMEOUT_SECONDS: u64 = 300;
