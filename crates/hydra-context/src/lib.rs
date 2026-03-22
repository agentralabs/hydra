//! `hydra-context` — Five windows of situational awareness.
//!
//! Provides active, historical, predicted, gap, and anomaly context
//! windows that combine into a `ContextFrame` for Hydra's decision-making.

pub mod active;
pub mod anomaly;
pub mod constants;
pub mod errors;
pub mod frame;
pub mod gap;
pub mod historical;
pub mod predicted;
pub mod window;

pub use active::build_active;
pub use anomaly::{AnomalyContext, AnomalySignal};
pub use errors::ContextError;
pub use frame::ContextFrame;
pub use gap::{GapContext, GapSignal};
pub use historical::{build_historical, SessionHistory};
pub use predicted::{build_predicted, StagedIntent};
pub use window::{ContextItem, ContextWindow};
