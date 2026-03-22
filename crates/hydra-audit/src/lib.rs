//! `hydra-audit` — Execution accountability narrative.
//!
//! Receipts are cryptographic primitives.
//! This crate makes them human-readable.
//!
//! "12 attempts, 3 reroutes, 2 escalations, 1 completion.
//!  Here is exactly what happened and why."
//!
//! Immutable records. Integrity-hashed. Queryable forever.
//! Settlement reads from here. Attribution reads from here.
//! The entity accounts for everything it does.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod event;
pub mod narrative;
pub mod persistence;
pub mod record;
pub mod trace;

pub use engine::AuditEngine;
pub use errors::AuditError;
pub use event::{EventKind, TraceEvent};
pub use narrative::{ExecutionNarrative, NarrativeBuilder};
pub use record::{AuditQuery, AuditRecord, AuditStore};
pub use trace::ExecutionTrace;
