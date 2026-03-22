//! Constants for oracle projections.

/// Maximum number of scenarios per projection.
pub const MAX_SCENARIOS_PER_PROJECTION: usize = 5;

/// Minimum probability threshold for a scenario to be included.
pub const MIN_SCENARIO_PROBABILITY: f64 = 0.05;

/// Probability threshold above which a scenario is considered adverse.
pub const ADVERSE_PROBABILITY_THRESHOLD: f64 = 0.3;

/// Base probability for risk-derived scenarios.
pub const RISK_BASE_PROBABILITY: f64 = 0.35;

/// Base probability for optimization-derived scenarios.
pub const OPTIMIZATION_BASE_PROBABILITY: f64 = 0.40;

/// Base probability for causal-link cascade scenarios.
pub const CASCADE_BASE_PROBABILITY: f64 = 0.25;

/// Base probability for default scenarios.
pub const DEFAULT_BASE_PROBABILITY: f64 = 0.20;

/// Base confidence for projections with few primitives.
pub const LOW_PRIMITIVE_CONFIDENCE: f64 = 0.50;

/// Base confidence for projections with many primitives.
pub const HIGH_PRIMITIVE_CONFIDENCE: f64 = 0.75;

/// Primitive count threshold separating low from high confidence.
pub const PRIMITIVE_COUNT_THRESHOLD: usize = 3;
