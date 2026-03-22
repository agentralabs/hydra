//! `hydra-morphic` — Morphic identity for Hydra.
//!
//! This crate provides:
//! - A continuous, unforgeable identity signature (hash chain)
//! - Morphic event recording and history
//! - Identity distance computation for entity comparison
//! - Restart tracking with signature continuity

pub mod constants;
pub mod errors;
pub mod event;
pub mod identity;
pub mod signature;

pub use errors::MorphicError;
pub use event::{MorphicEvent, MorphicEventKind};
pub use identity::MorphicIdentity;
pub use signature::MorphicSignature;
