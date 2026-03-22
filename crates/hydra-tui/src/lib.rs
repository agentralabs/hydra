//! `hydra-tui` — The cockpit. Hydra's primary interface.
//!
//! This crate provides the TUI rendering engine for Hydra.
//! The OutputPacer governs ALL output — nothing bypasses it.
//! Thinking verb colors are PERMANENT — defined once in constants.

pub mod app;
pub mod cockpit;
pub mod constants;
pub mod dot;
pub mod errors;
pub mod input;
pub mod pacer;
pub mod status;
pub mod stream;
pub mod stream_types;
pub mod render_cockpit;
pub mod render_welcome;
pub mod verb;
pub mod welcome;

// Re-exports for convenience.
pub use app::HydraTui;
pub use cockpit::{CockpitMode, CockpitView};
pub use constants::{ALL_DOT_COLORS, ALL_VERB_COLORS};
pub use dot::DotKind;
pub use errors::TuiError;
pub use input::InputBox;
pub use pacer::{ContentKind, OutputPacer, PacerSignals};
pub use status::StatusLine;
pub use stream::ConversationStream;
pub use stream_types::{BriefingPriority, CompanionStatus, StreamItem};
pub use verb::{ThinkingVerbState, VerbContext};
pub use welcome::WelcomeScreen;
