//! `hydra-omniscience` — Active knowledge acquisition.
//!
//! Every other AI system: "I don't know."
//! Hydra: "I don't know yet. Acquiring."
//!
//! Gap detection -> acquisition plan -> multiple sources ->
//! belief integration -> gap closed. Permanently.
//!
//! The acquired knowledge enters the belief manifold.
//! Next time Hydra encounters this topic: already knows.
//! Recurring gaps flag domain for skill loading.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod gap;
pub mod persistence;
pub mod plan;
pub mod result;
pub mod source;

pub use engine::{AcquisitionSummary, OmniscienceEngine};
pub use errors::OmniscienceError;
pub use gap::{GapState, GapType, KnowledgeGap};
pub use plan::AcquisitionPlan;
pub use result::AcquisitionResult;
pub use source::AcquisitionSource;
