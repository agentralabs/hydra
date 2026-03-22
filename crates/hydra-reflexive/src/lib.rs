//! `hydra-reflexive` — Hydra's self-model and safe self-modification engine.
//!
//! This crate provides:
//! - A runtime self-model of all known capabilities
//! - Constitutional-check-gated self-modification
//! - Snapshot-based rollback for failed modifications
//! - Growth invariant enforcement (capabilities never decrease)

pub mod capability;
pub mod constants;
pub mod errors;
pub mod model;
pub mod modifier;
pub mod snapshot;

pub use capability::{CapabilityNode, CapabilitySource, CapabilityStatus};
pub use errors::ReflexiveError;
pub use model::SelfModel;
pub use modifier::{ModificationKind, ModificationProposal, SafeModifier};
pub use snapshot::SelfSnapshot;
