//! All constants for hydra-swarm.
//! No magic numbers or strings anywhere else in this crate.

/// Minimum number of agents required for consensus.
pub const CONSENSUS_MIN_AGENTS: usize = 2;

/// Jaccard similarity threshold for grouping answers as "same".
pub const CONSENSUS_SIMILARITY_THRESHOLD: f64 = 0.75;

/// Maximum number of emergence entries stored.
pub const EMERGENCE_MAX_ENTRIES: usize = 10_000;

/// Minimum fraction of active agents for a healthy swarm.
pub const SWARM_HEALTH_MIN_ACTIVE_FRACTION: f64 = 0.6;

/// Lyapunov bonus applied on positive consensus.
pub const SWARM_LYAPUNOV_BONUS: f64 = 0.05;

/// Lyapunov penalty applied on degraded health.
pub const SWARM_LYAPUNOV_PENALTY: f64 = 0.1;
