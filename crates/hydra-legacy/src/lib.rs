//! `hydra-legacy` — Permanent knowledge export.
//!
//! What Hydra learned escapes to the world.
//!
//! Three kinds of legacy artifact:
//!   Knowledge records (soul orientation distilled)
//!   Operational records (proven approaches over time)
//!   Wisdom records (cross-domain calibration insight)
//!
//! Every artifact is integrity-verified (SHA256) and immutable once stored.
//! Layer 7, Phase 2: the knowledge survives the entity.

pub mod artifact;
pub mod builder;
pub mod constants;
pub mod engine;
pub mod errors;

pub use artifact::{LegacyArtifact, LegacyKind};
pub use builder::LegacyBuilder;
pub use engine::LegacyEngine;
pub use errors::LegacyError;
