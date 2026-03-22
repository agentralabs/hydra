/// Maximum knowledge gaps tracked simultaneously.
pub const MAX_TRACKED_GAPS: usize = 10_000;

/// Maximum sources in one acquisition plan.
pub const MAX_SOURCES_PER_PLAN: usize = 5;

/// Minimum confidence for acquired knowledge to enter belief manifold.
pub const MIN_ACQUISITION_CONFIDENCE: f64 = 0.50;

/// Source reliability scores (0.0-1.0).
pub const SOURCE_RELIABILITY_AGENTIC_CODEBASE: f64 = 0.88;
pub const SOURCE_RELIABILITY_DOCUMENTATION:    f64 = 0.82;
pub const SOURCE_RELIABILITY_EXPERT_SYSTEM:    f64 = 0.79;
pub const SOURCE_RELIABILITY_BELIEF_SYNTHESIS: f64 = 0.65;
pub const SOURCE_RELIABILITY_WEB:              f64 = 0.60;

/// Recurring gap threshold — how many times a gap recurs
/// before flagging the domain for skill loading.
pub const RECURRING_GAP_THRESHOLD: usize = 3;

/// Maximum acquisition results stored.
pub const MAX_ACQUISITION_RESULTS: usize = 50_000;
