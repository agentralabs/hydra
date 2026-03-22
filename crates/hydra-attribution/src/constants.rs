/// Fraction of cost attributed to rerouting before it's flagged.
pub const REROUTING_COST_FLAG_THRESHOLD: f64 = 0.20;

/// Fraction of cost attributed to knowledge acquisition
/// before it signals a missing skill.
pub const KNOWLEDGE_COST_SKILL_SIGNAL_THRESHOLD: f64 = 0.25;

/// Maximum attribution tree depth.
pub const MAX_ATTRIBUTION_DEPTH: usize = 8;

/// Maximum attribution records stored.
pub const MAX_ATTRIBUTION_RECORDS: usize = 500_000;

/// Avoidable cost fraction threshold — surfaces to portfolio.
pub const AVOIDABLE_COST_ALERT_THRESHOLD: f64 = 0.30;
