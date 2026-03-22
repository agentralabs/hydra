//! `hydra-metabolism` — Lyapunov stability enforcement.
//!
//! The long-term health guardian. This crate provides:
//! - Lyapunov value tracking and stability classification
//! - Intervention events triggered by stability degradation
//! - A tick-based monitor that enforces the growth invariant
//! - Metabolism reports for display

pub mod constants;
pub mod errors;
pub mod intervention;
pub mod lyapunov;
pub mod monitor;
pub mod report;

pub use errors::MetabolismError;
pub use intervention::{InterventionEvent, InterventionLevel};
pub use lyapunov::{LyapunovTracker, StabilityClass};
pub use monitor::MetabolismMonitor;
pub use report::MetabolismReport;
