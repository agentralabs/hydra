//! Global application state — messages, phases, connection, runs.
//!
//! Split into submodules for maintainability:
//! - `hydra_types`: CognitivePhase, GlobeState, AppConfig, and supporting enums/structs
//! - `hydra_state`: HydraState impl
//! - `hydra_tests`: tests

// Re-export everything at the original path
pub use super::hydra_types::*;
pub use super::hydra_state::HydraState;
