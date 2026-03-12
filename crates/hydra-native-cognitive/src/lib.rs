pub mod cognitive;
pub mod environment;
pub mod knowledge;
pub mod project_exec;
pub mod remote;
pub mod sister_improve;
pub mod sisters;
pub mod threat;
pub mod swarm;
pub mod task_persistence;
pub mod tools;

// Re-exports
pub use cognitive::{AgentSpawner, CapabilityRegistry, CognitiveLoopConfig, CognitiveUpdate, DecideEngine, DecideResult, InventionEngine, RuntimeSettings, run_cognitive_loop};
pub use cognitive::streaming::{StreamBuffer, StreamingConfig, StreamState};
pub use sisters::{init_sisters, Sisters, SistersHandle};
