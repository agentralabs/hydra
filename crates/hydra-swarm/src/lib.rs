//! `hydra-swarm` — Collective intelligence for the Hydra fleet.
//!
//! Emergence detection, consensus evaluation, and swarm health monitoring.

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
