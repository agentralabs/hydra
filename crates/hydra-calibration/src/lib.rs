//! `hydra-calibration` — Epistemic calibration.
//!
//! Hydra knows where its judgment goes wrong.
//!
//! Raw confidence: what Hydra computes.
//! Calibrated confidence: what Hydra honestly reports.
//!
//! "My raw confidence is 0.83.
//!  I have a known +0.11 overconfidence bias here.
//!  Calibrated confidence: 0.72."
//!
//! Without this: confident answers, some systematically wrong.
//! With this: honest answers, the principal knows when to trust less.

pub mod adjuster;
pub mod bias;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod persistence;
pub mod record;

pub use adjuster::{AdjustedConfidence, ConfidenceAdjuster};
pub use bias::{BiasEntry, BiasKey, BiasProfiler};
pub use engine::CalibrationEngine;
pub use errors::CalibrationError;
pub use record::{CalibrationRecord, JudgmentType, PredictionOutcome};
