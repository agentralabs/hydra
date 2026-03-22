//! `hydra-comprehension` — The gateway to intelligence.
//!
//! Lifts any input to structured meaning via a 4-stage pipeline:
//! domain detection, primitive mapping, temporal placement, and
//! memory resonance. Zero LLM calls — pure structural analysis.

pub mod constants;
pub mod domain;
pub mod engine;
pub mod errors;
pub mod loop_bridge;
pub mod output;
pub mod primitive;
pub mod resonance;
pub mod temporal;

pub use domain::{Domain, DomainVocabulary};
pub use engine::{meets_threshold, ComprehensionEngine};
pub use errors::ComprehensionError;
pub use output::{ComprehendedInput, InputSource};
pub use primitive::PrimitiveMapping;
pub use resonance::{MemoryResonance, ResonanceMatch, ResonanceResult};
pub use temporal::{ConstraintStatus, Horizon, TemporalContext, TemporalPlacement};
