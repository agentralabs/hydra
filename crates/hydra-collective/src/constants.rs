/// Minimum observations (from any peers combined) to produce an insight.
pub const MIN_OBSERVATIONS_FOR_INSIGHT: usize = 3;

/// Minimum aggregated confidence to publish an insight.
pub const MIN_INSIGHT_CONFIDENCE: f64 = 0.65;

/// Trust weight — higher trust peers contribute more to aggregation.
pub const TRUST_WEIGHT_EXPONENT: f64 = 1.5;

/// Maximum observations stored per pattern topic.
pub const MAX_OBSERVATIONS_PER_TOPIC: usize = 10_000;

/// Maximum collective insights stored.
pub const MAX_STORED_INSIGHTS: usize = 50_000;
