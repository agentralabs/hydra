//! `hydra-consent` — Fine-grained sharing consent.
//!
//! Specific. Versioned. Revocable. Audited. Time-bounded.
//!
//! No consent -> no sharing. Hard stop.
//! Every share event references the consent that authorized it.
//! Revoke at any time. Takes effect immediately.

pub mod audit;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod grant;
pub mod registry;

pub use audit::{ConsentAuditEntry, ConsentAuditLog};
pub use engine::ConsentEngine;
pub use errors::ConsentError;
pub use grant::{ConsentGrant, ConsentScope, GrantState};
pub use registry::ConsentRegistry;
