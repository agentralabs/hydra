/// Confidence gap that triggers full consensus resolution.
/// Below this: the higher confidence wins. Above: full arbiter.
pub const CONSENSUS_TRIGGER_GAP: f64 = 0.10;

/// Evidence weight relative to confidence in final merge.
pub const EVIDENCE_WEIGHT: f64 = 0.40;
/// Confidence weight in final merge.
pub const CONFIDENCE_WEIGHT: f64 = 0.60;

/// Minimum merged confidence before result is flagged uncertain.
pub const MIN_MERGED_CONFIDENCE: f64 = 0.50;

/// Maximum beliefs resolved per consensus session.
pub const MAX_BELIEFS_PER_SESSION: usize = 1_000;
