//! `hydra-crystallizer` — Wisdom into reusable artifacts.
//!
//! NOT templates. Operational history.
//!
//! 8 successful deployments -> Deployment Playbook.
//! 4 repeated failures -> Post-Mortem with root causes.
//! 20 knowledge acquisitions -> Knowledge Base.
//!
//! Generated from what actually happened.
//! The difference between a manual and a memoir.

pub mod artifact;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod generator;
pub mod source;

pub use artifact::{ArtifactKind, CrystallizedArtifact};
pub use engine::CrystallizerEngine;
pub use errors::CrystallizerError;
pub use generator::ArtifactGenerator;
pub use source::CrystallizationSource;
