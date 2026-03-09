//! Cognitive loop — decoupled from UI via message passing.

pub mod decide;
pub mod intent_router;
pub mod inventions;
pub mod loop_runner;
pub mod omniscience;
pub mod self_repair;
pub mod spawner;
pub mod streaming;

pub use decide::{DecideEngine, DecideResult};
pub use inventions::InventionEngine;
pub use loop_runner::{CognitiveLoopConfig, CognitiveUpdate, run_cognitive_loop};
pub use omniscience::{OmniscienceEngine, OmniscienceGap, OmniscienceScan, OmniscienceUpdate, RepoTarget, RepoScan};
pub use self_repair::{SelfRepairEngine, RepairSpec, RepairResult, RepairStatus, RepairUpdate};
pub use spawner::AgentSpawner;
