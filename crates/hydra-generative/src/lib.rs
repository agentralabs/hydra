//! `hydra-generative` — Synthesize new capabilities from axiom primitives.
//! The capability ceiling is mathematical infinity.

pub mod compose;
pub mod constants;
pub mod decompose;
pub mod engine;
pub mod errors;
pub mod gap;

pub use compose::CompositionResult;
pub use decompose::TaskDecomposition;
pub use engine::{GenerativeEngine, SynthesisOutcome};
pub use errors::GenerativeError;
pub use gap::{detect_gap, CapabilityGap};
