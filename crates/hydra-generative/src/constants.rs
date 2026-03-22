//! Constants for the generative crate.

/// Minimum confidence threshold for synthesized capabilities.
pub const SYNTHESIS_MIN_CONFIDENCE: f64 = 0.3;

/// Maximum number of axiom primitives in a single decomposition.
pub const MAX_DECOMPOSITION_PRIMITIVES: usize = 10;

/// Hydra never says "cannot" — gaps are specific about what is needed.
pub const GAP_NEVER_SAYS_CANNOT: bool = true;
