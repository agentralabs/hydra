//! `hydra-oracle` — Probabilistic future modeling.
//!
//! Generates scenario projections from axiom primitives, assigning
//! probabilities and identifying adverse outcomes. Each primitive type
//! maps to a scenario archetype: Risk produces adverse scenarios,
//! Optimization produces positive scenarios, CausalLink produces
//! cascade scenarios.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod projection;
pub mod scenario;

pub use engine::OracleEngine;
pub use errors::OracleError;
pub use projection::OracleProjection;
pub use scenario::Scenario;
