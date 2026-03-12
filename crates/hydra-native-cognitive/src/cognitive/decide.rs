//! DECIDE phase — graduated autonomy + execution gate + anomaly detection.
//!
//! Split into sub-modules by responsibility:
//! - `decide_challenge` — challenge phrase gate for irreversible HIGH+ risk actions
//! - `decide_anomaly` — anomaly detection (burst, destructive patterns, exfiltration)
//! - `decide_engine` — DecideEngine orchestrating the full 6-layer security pipeline
//! - `decide_tests` — all unit and integration tests

// Re-export everything so existing `crate::cognitive::decide::X` paths keep working.
pub use super::decide_anomaly::{AnomalyDetector, CommandGateResult, DecideResult};
pub use super::decide_challenge::{generate_challenge_phrase, ChallengePhraseGate};
pub use super::decide_engine::DecideEngine;
