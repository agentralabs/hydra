/// Maximum jobs in the queue.
pub const MAX_QUEUED_JOBS: usize = 10_000;

/// Maximum jobs firing simultaneously.
pub const MAX_CONCURRENT_FIRES: usize = 10;

/// How far ahead to look for due jobs (seconds).
pub const LOOKAHEAD_SECONDS: i64 = 60;

/// Minimum interval between the same recurring job fires (seconds).
pub const MIN_RECURRING_INTERVAL_SECONDS: u64 = 30;

/// How long a failed job stays before retry (seconds).
pub const FAILED_JOB_RETRY_SECONDS: u64 = 300;

/// Maximum retries for a job before it is suspended.
pub const MAX_JOB_RETRIES: u32 = 5;

/// Schedule state persistence key (for resurrection).
pub const SCHEDULE_STATE_KEY: &str = "hydra:scheduler:state";
