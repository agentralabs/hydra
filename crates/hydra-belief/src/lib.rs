//! `hydra-belief` — AGM belief revision as geodesic flow on the manifold.
//!
//! Implements the AGM postulates for rational belief change,
//! with the key invariant that capability beliefs (Protected policy)
//! can never be revised downward.

pub mod belief;
pub mod constants;
pub mod errors;
pub mod manifold;
pub mod postulates;
pub mod revision;
pub mod store;

pub use belief::{Belief, BeliefCategory, RevisionPolicy};
pub use errors::BeliefError;
pub use manifold::BeliefPosition;
pub use postulates::{verify_consistency, verify_inclusion, verify_success};
pub use revision::{proposition_overlap, revise, RevisionResult};
pub use store::BeliefStore;
