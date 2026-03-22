//! `hydra-genome` — Capability genetics.
//!
//! Tracks situation-approach pairs with confidence scores,
//! enabling Hydra to reuse successful strategies. The genome store
//! is append-only: entries are never deleted.

pub mod constants;
pub mod entry;
pub mod errors;
pub mod persistence;
pub mod signature;
pub mod skill_loader;
pub mod store;

pub use entry::GenomeEntry;
pub use errors::GenomeError;
pub use signature::{ApproachSignature, SituationSignature};
pub use store::GenomeStore;
