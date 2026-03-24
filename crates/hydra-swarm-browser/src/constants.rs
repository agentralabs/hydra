//! Configurable constants for swarm browser operations.

pub const DEFAULT_POOL_SIZE: usize = 5;
pub const MAX_POOL_SIZE: usize = 20;
pub const WORKER_TIMEOUT_SECS: u64 = 120;
pub const MAX_SUBTASKS_PER_GOAL: usize = 10;
pub const MIN_CONTENT_WORDS: usize = 50;
pub const YOUTUBE_TRANSCRIPT_WAIT_MS: u64 = 3000;
pub const MERGE_DEDUP_SIMILARITY: f64 = 0.80;
pub const MIN_CONSENSUS_WORKERS: usize = 2;
