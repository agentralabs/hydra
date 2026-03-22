//! `hydra-attribution` — Causal cost tracing.
//!
//! "This deployment cost 47K tokens. Here is why:
//!  31K: security review triggered by unexpected trust boundary change.
//!  12K: first-time operation on new cloud provider (one-time cost).
//!  4K: rerouting from concurrent lock (avoidable — coordination issue)."
//!
//! NOT what things cost — that is hydra-settlement.
//! WHY they cost what they did.
//! The difference between accounting and intelligence.

pub mod avoidable;
pub mod cause;
pub mod constants;
pub mod cost;
pub mod engine;
pub mod errors;
pub mod tree;

pub use avoidable::AvoidabilityReport;
pub use cause::{infer_factors, CausalFactor, CausalFactorType};
pub use cost::{CostClass, CostItem};
pub use engine::AttributionEngine;
pub use errors::AttributionError;
pub use tree::AttributionTree;
