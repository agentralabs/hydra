//! `hydra-companion` — Signal stream and task executor.
//!
//! The companion system classifies incoming signals by urgency
//! and manages background tasks. All companion actions are
//! ALWAYS visible in the TUI stream.
//!
//! Architecture: Companion runs independently, communicates with the TUI
//! through hydra-signals CompanionChannel. Neither depends on the other.

pub mod companion;
pub mod constants;
pub mod errors;
pub mod service;
pub mod signal;
pub mod task;

// Re-exports for convenience.
pub use companion::{Companion, CompanionCommand, RoutedSignal};
pub use errors::CompanionError;
pub use service::CompanionService;
pub use signal::{SignalBuffer, SignalClass, SignalClassifier, SignalItem, SignalRouting};
pub use task::{AutonomyLevel, CompanionTask, TaskExecutor, TaskStatus};
