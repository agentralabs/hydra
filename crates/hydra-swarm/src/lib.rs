//! `hydra-swarm` — Collective intelligence for the Hydra fleet.
//!
//! Emergence detection, consensus evaluation, and swarm health monitoring.
//!
//! INCOMPLETE (Session 21): Swarm learning is not yet implemented.
//! Missing: browser pool for parallel web harvesting, task decomposer that splits
//! a learning goal into sub-queries, and swarm result merger that deduplicates and
//! merges genome entries from multiple browser agents.
//! Estimated ~200 lines across 3 new modules. See specs/HYDRA-AUTONOMOUS-LEARNING.md Phase 5.
//! IMPORTANT: Enter plan mode before implementing. Design the browser pool lifecycle,
//! resource limits (EC-20.5: OOM on 8GB machines), and consensus threshold first.

pub mod consensus;
pub mod constants;
pub mod emergence;
pub mod errors;
pub mod health;
pub mod swarm;

pub use consensus::{detect_consensus, AgentAnswer, ConsensusSignal};
pub use emergence::{EmergenceEntry, EmergenceStore};
pub use errors::SwarmError;
pub use health::{SwarmHealth, SwarmHealthLevel};
pub use swarm::Swarm;
