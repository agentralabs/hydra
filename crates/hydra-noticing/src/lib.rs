//! `hydra-noticing` — Ambient observation without prompt.
//!
//! "I noticed deployment latency has increased 3% per week
//!  for 6 weeks. Nobody asked me to check."
//!
//! No system asked for this. No rule was set.
//! Hydra noticed because it is always alive
//! and always watching.
//!
//! This is the final Layer 2 crate.
//! When Phase 21 passes, Layer 2 is complete.

pub mod baseline;
pub mod compound;
pub mod constants;
pub mod drift;
pub mod engine;
pub mod errors;
pub mod pattern;
pub mod signal;
pub mod surprise;

pub use compound::{CompoundRiskDetector, SmallIssue};
pub use surprise::{SurpriseDetector, SurpriseEvent};
pub use engine::NoticingEngine;
pub use errors::NoticingError;
pub use signal::{DriftDirection, NoticingKind, NoticingSignal};
