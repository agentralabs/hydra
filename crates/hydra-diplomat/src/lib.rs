//! `hydra-diplomat` — Multi-instance coordination.
//!
//! No participant is "in charge."
//! Every stance is contributed independently.
//! The joint recommendation emerges from synthesis.
//! Minority positions are preserved — never suppressed.
//! Disagreement is a signal, not a bug.
//!
//! Layer 6 closes here.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod session;
pub mod stance;
pub mod synthesis;

pub use engine::DiplomatEngine;
pub use errors::DiplomatError;
pub use session::{DiplomacySession, SessionState};
pub use stance::Stance;
pub use synthesis::{synthesize, JointRecommendation};
