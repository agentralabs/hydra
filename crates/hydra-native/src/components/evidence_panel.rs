//! Evidence panel component data — code preview, screenshots, memory context.
//!
//! Split into submodules for maintainability:
//! - `evidence_panel_core`: types, struct definition, and all impl methods
//! - `evidence_panel_tests`: unit tests

#[path = "evidence_panel_core.rs"]
mod evidence_panel_core;

#[cfg(test)]
#[path = "evidence_panel_tests.rs"]
mod evidence_panel_tests;

pub use evidence_panel_core::{EvidenceItem, EvidenceKind, EvidencePanel};
