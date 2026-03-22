//! `hydra-learning` — Reasoning weight evolution.
//!
//! Observes which reasoning modes produce accurate conclusions in each
//! domain and proposes weight adjustments via `LearningRecord` proposals.
//! This crate is an observer — it never modifies reasoning weights directly.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod observation;
pub mod record;
pub mod tracker;

pub use engine::LearningEngine;
pub use errors::LearningError;
pub use observation::{ObservationOutcome, ReasoningObservation};
pub use record::LearningRecord;
pub use tracker::ModeTracker;
