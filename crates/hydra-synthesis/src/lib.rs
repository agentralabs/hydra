//! `hydra-synthesis` — Cross-domain pattern discovery.
//!
//! Discovers structural similarities across domains by comparing axiom
//! primitive signatures. Never calls the LLM — all insights are derived
//! from pattern matching over primitives and genome patterns.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod insight;
pub mod matcher;
pub mod pattern;

pub use engine::SynthesisEngine;
pub use errors::SynthesisError;
pub use insight::SynthesisInsight;
pub use matcher::{find_cross_domain_matches, CrossDomainMatch};
pub use pattern::StructuralPattern;
