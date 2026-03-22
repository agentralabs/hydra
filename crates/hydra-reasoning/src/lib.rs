//! `hydra-reasoning` — Five reasoning modes simultaneously.
//!
//! Runs deductive, inductive, abductive, analogical, and adversarial
//! reasoning in parallel. The first mode that concludes contributes
//! to the synthesized result. Over 85% of reasoning cycles complete
//! with zero LLM calls.

pub mod abductive;
pub mod adversarial;
pub mod analogical;
pub mod conclusion;
pub mod constants;
pub mod deductive;
pub mod engine;
pub mod errors;
pub mod inductive;
pub mod introspection;

pub use conclusion::{ReasoningConclusion, ReasoningMode};
pub use introspection::{IntrospectionConfig, IntrospectionResult, introspect_confidence};
pub use engine::{ReasoningEngine, ReasoningResult};
pub use errors::ReasoningError;
pub use inductive::SituationSignatureExt;
