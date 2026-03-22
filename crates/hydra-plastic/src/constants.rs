//! Constants for the plastic crate.

/// Maximum number of environment profiles.
pub const MAX_ENVIRONMENTS: usize = 10_000;

/// Confidence boost per successful encounter.
pub const ENVIRONMENT_CONFIDENCE_BOOST: f64 = 0.03;

/// Default confidence for a new environment.
pub const DEFAULT_CONFIDENCE: f64 = 0.5;
