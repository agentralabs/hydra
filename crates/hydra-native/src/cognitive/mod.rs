//! Cognitive loop — decoupled from UI via message passing.

pub mod loop_runner;
pub mod streaming;

pub use loop_runner::{CognitiveLoopConfig, CognitiveUpdate, run_cognitive_loop};
