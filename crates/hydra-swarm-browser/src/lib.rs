//! hydra-swarm-browser — Distributed multi-agent web intelligence.
//!
//! Spawns N browser workers simultaneously across the internet.
//! Decomposes goals, extracts content, watches YouTube, reads docs,
//! merges knowledge, checks consensus, stores in genome.

pub mod constants;
pub mod decomposer;
pub mod merger;
pub mod orchestrator;
pub mod types;
pub mod worker;
pub mod youtube;

pub use orchestrator::{execute_swarm, execute_swarm_blocking, spawn_swarm};
pub use types::{SwarmGoal, SwarmResponse, SwarmTask, SwarmUpdate, WorkerResult};
