//! hydra-semantic-nav — Semantic Affordance Navigation.
//!
//! The invention: instead of screenshots → vision → coordinates,
//! parse DOM → extract affordances → match intent → CDP.interact(element).
//! 50ms vs 5,000ms. Zero LLM tokens. Structurally stable.

pub mod affordance;
pub mod constitution_cache;
pub mod dom_parser;
pub mod executor;
pub mod intent_router;
pub mod orchestrator;
pub mod types;
pub mod verifier;

pub use orchestrator::{try_semantic_nav, try_semantic_nav_with_url};
pub use types::{ExecutionPlan, NavResult, PageConstitution, SemanticElement};
