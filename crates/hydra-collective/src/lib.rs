//! `hydra-collective` — Distributed pattern intelligence.
//!
//! One Hydra sees a pattern 3 times.
//! Ten federated Hydras see it 47 times combined.
//! The pattern is now proven, not suspected.
//!
//! P2P. No central registry. Trust-weighted. Consent-gated.

pub mod aggregator;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod insight;
pub mod observation;

pub use aggregator::{aggregate, AggregatedPattern};
pub use engine::CollectiveEngine;
pub use errors::CollectiveError;
pub use insight::CollectiveInsight;
pub use observation::PatternObservation;
