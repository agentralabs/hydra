/// Weight of risk reduction in objective scoring.
pub const WEIGHT_RISK_REDUCTION: f64 = 0.30;
/// Weight of orientation alignment in objective scoring.
pub const WEIGHT_ORIENTATION: f64 = 0.25;
/// Weight of return-on-investment in objective scoring.
pub const WEIGHT_ROI: f64 = 0.25;
/// Weight of urgency in objective scoring.
pub const WEIGHT_URGENCY: f64 = 0.20;

/// Maximum objectives in one portfolio.
pub const MAX_PORTFOLIO_OBJECTIVES: usize = 50;

/// Minimum score to recommend an objective.
pub const MIN_RECOMMENDATION_SCORE: f64 = 0.30;

/// Attention budget denominated in arbitrary units.
pub const DEFAULT_ATTENTION_BUDGET: f64 = 100.0;
