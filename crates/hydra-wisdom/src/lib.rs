//! `hydra-wisdom` — Where intelligence becomes judgment.
//!
//! Layer 4 closes here.
//!
//! "The data says X. But pattern history says be careful.
//!  Three times before when data said X it was wrong
//!  because of Y. Recommend verifying Y before acting."
//!
//! That is not computation.
//! That is judgment.
//! That is what this crate produces.

pub mod constants;
pub mod distillation;
pub mod engine;
pub mod errors;
pub mod input;
pub mod memory;
pub mod persistence;
pub mod statement;
pub mod uncertainty;

pub use distillation::{Archetype, MetaPattern, WisdomDistiller};
pub use engine::WisdomEngine;
pub use uncertainty::{UncertaintyNode, UncertaintyTree};
pub use errors::WisdomError;
pub use input::{
    CalibrationEvidence, OracleEvidence, PatternEvidence, RedTeamEvidence, WisdomInput,
};
pub use memory::{JudgmentOutcome, WisdomMemory, WisdomMemoryEntry};
pub use statement::{Recommendation, WisdomStatement};
