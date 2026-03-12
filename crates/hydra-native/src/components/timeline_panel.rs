//! Timeline panel component data — chronological events with timestamps and phase indicators.
//!
//! Split into submodules for maintainability:
//! - `timeline_panel_core`: types, struct definition, and all impl methods
//! - `timeline_panel_tests`: unit tests

#[path = "timeline_panel_core.rs"]
mod timeline_panel_core;

#[path = "timeline_panel_tests.rs"]
mod timeline_panel_tests;

pub use timeline_panel_core::{TimelineEvent, TimelineEventKind, TimelinePanel};
