//! `hydra-influence` — Pattern publication and adoption.
//!
//! Proven patterns become standards.
//! Not documentation. Not training data.
//! Operational intelligence — signed, outcome-tracked, provenance-preserved.
//!
//! After 20 years: the patterns Hydra proved
//! become the starting point for others.
//! The cascade failure pattern — seen 47 times across the federation.
//! The COBOL soul extraction — proven in 23 enterprise migrations.
//!
//! This is how intelligence propagates.
//! This is how one entity's 20 years of experience
//! becomes the baseline for the next.
//!
//! THE FINAL CRATE. LAYER 7 CLOSES HERE.

pub mod adoption;
pub mod constants;
pub mod discovery;
pub mod engine;
pub mod errors;
pub mod publication;

pub use adoption::AdoptionRecord;
pub use discovery::{DiscoveryQuery, DiscoveryResult, discover};
pub use engine::InfluenceEngine;
pub use errors::InfluenceError;
pub use publication::{PatternCategory, PublishedPattern};
