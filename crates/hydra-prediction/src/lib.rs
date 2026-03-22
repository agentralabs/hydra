//! `hydra-prediction` — Predictive staging for Hydra.
//!
//! Hydra prepares before you ask. This crate maintains a prediction stage
//! of likely next intents, runs shadow executions, and detects divergence
//! between predicted and actual outcomes to trigger belief revision.

pub mod constants;
pub mod divergence;
pub mod errors;
pub mod intent;
pub mod shadow;
pub mod staging;

pub use divergence::DivergenceDetector;
pub use errors::PredictionError;
pub use intent::{IntentPrediction, PredictionBasis, PredictionStage};
pub use shadow::{compute_divergence, ActualOutcome, OutcomeDivergence, ShadowOutcome};
pub use staging::{PredictionStager, RecordedIntent};
