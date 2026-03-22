//! All constants for hydra-fleet.
//! No magic numbers or strings anywhere else in this crate.

/// Maximum number of agents allowed in a single fleet.
pub const FLEET_MAX_AGENTS: usize = 256;

/// Maximum number of tasks queued per agent.
pub const AGENT_MAX_TASK_QUEUE: usize = 16;

/// Default task timeout in seconds.
pub const TASK_TIMEOUT_SECONDS: u64 = 300;

/// Maximum size of a result content payload in bytes.
pub const RESULT_MAX_CONTENT_BYTES: usize = 1_048_576; // 1 MB

/// Minimum number of agents required for consensus.
pub const CONSENSUS_MIN_AGENTS: usize = 2;

/// Duration (seconds) a quarantined agent must wait before review.
pub const QUARANTINE_HOLD_SECONDS: u64 = 3600;

/// Minimum trust score required to spawn a new agent.
pub const SPAWN_MIN_TRUST_SCORE: f64 = 0.2;

/// Maximum length of an agent name.
pub const AGENT_NAME_MAX_LEN: usize = 64;
