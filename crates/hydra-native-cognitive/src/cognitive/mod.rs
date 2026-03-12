//! Cognitive loop — decoupled from UI via message passing.

pub mod capability_registry;
pub mod conversation_engine;
pub mod decide;
pub mod decide_anomaly;
pub mod decide_challenge;
pub mod decide_engine;
mod decide_tests;
pub mod handlers;
pub mod intent_router;
pub mod intent_router_classify;
mod intent_router_tests;
pub mod inventions;
pub mod learn;
pub mod loop_runner;
pub mod omniscience;
pub mod omniscience_loop;
pub mod omniscience_phases;
pub mod omniscience_scanners;
pub mod self_repair;
pub mod self_repair_loop;
pub mod spawner;
pub mod streaming;
pub mod obstacles;
pub mod runtime_settings;

pub use decide::{ChallengePhraseGate, DecideEngine, DecideResult, generate_challenge_phrase};
pub use learn::{apply_belief_decay, gc_expired_beliefs, reconfirm_belief};
pub use inventions::InventionEngine;
pub use loop_runner::{CognitiveLoopConfig, CognitiveUpdate, run_cognitive_loop};
pub use runtime_settings::RuntimeSettings;
pub use omniscience::{OmniscienceEngine, OmniscienceGap, OmniscienceScan, OmniscienceUpdate, RepoTarget, RepoScan};
pub use self_repair::{SelfRepairEngine, RepairSpec, RepairResult, RepairStatus, RepairUpdate};
pub use conversation_engine::{ConversationBuffer, ConversationContext};
pub use spawner::AgentSpawner;
pub use obstacles::{ObstacleResolver, Obstacle, ObstaclePattern, Resolution, ResolverConfig};
pub use capability_registry::CapabilityRegistry;
