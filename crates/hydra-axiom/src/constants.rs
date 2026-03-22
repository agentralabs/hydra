//! Constants for the axiom crate.

/// Maximum number of domain functors that can be registered.
pub const MAX_DOMAIN_FUNCTORS: usize = 256;

/// Maximum number of axiom primitives in a single composition.
pub const MAX_AXIOM_PRIMITIVES: usize = 1024;

/// Threshold for cross-domain similarity detection.
pub const CROSS_DOMAIN_SIMILARITY_THRESHOLD: f64 = 0.7;

/// Minimum confidence floor for synthesized capabilities.
pub const SYNTHESIS_CONFIDENCE_FLOOR: f64 = 0.3;
