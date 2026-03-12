pub mod cognitive;
pub mod environment;
pub mod knowledge;
pub mod project_exec;
pub mod sisters;
pub mod tools;

// Re-exports
pub use cognitive::{AgentSpawner, CognitiveLoopConfig, CognitiveUpdate, DecideEngine, DecideResult, InventionEngine, run_cognitive_loop};
pub use cognitive::streaming::{StreamBuffer, StreamingConfig, StreamState};
pub use sisters::{init_sisters, Sisters, SistersHandle};
