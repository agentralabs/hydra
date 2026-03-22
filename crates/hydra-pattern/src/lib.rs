//! `hydra-pattern` -- Deep pattern library.
//!
//! Anti-patterns: detect them BEFORE they fail.
//! Success patterns: apply them BEFORE trying from scratch.
//!
//! Domain-agnostic. Expressed at the axiom level.
//! A cascade failure in engineering is the same pattern
//! as a volatility cascade in finance.
//! The circuit breaker that fixes one fixes the other.

pub mod classifier;
pub mod constants;
pub mod engine;
pub mod entry;
pub mod errors;
pub mod matcher;

pub use classifier::{classify, ClassificationResult};
pub use engine::PatternEngine;
pub use entry::{PatternEntry, PatternKind};
pub use errors::PatternError;
pub use matcher::{find_matches, find_warnings};
