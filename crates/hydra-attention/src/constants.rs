//! Constants for the hydra-attention crate.
//!
//! All tunable values live here. No magic numbers in logic files.

/// Cost of full-depth processing for a single item.
pub const FULL_DEPTH_COST: u32 = 10;

/// Cost of summary-depth processing for a single item.
pub const SUMMARY_COST: u32 = 2;

/// Minimum significance score for an item to receive any attention.
pub const MIN_SIGNIFICANCE_FOR_ATTENTION: f64 = 0.15;

/// Minimum significance score for an item to receive full-depth processing.
pub const MIN_SIGNIFICANCE_FOR_FULL_DEPTH: f64 = 0.55;

/// Maximum number of items that can be in full-depth focus.
pub const MAX_FOCUS_ITEMS: usize = 12;

/// Maximum number of items that can be in summary depth.
pub const MAX_SUMMARY_ITEMS: usize = 20;

// --- Budget per intent kind (focus units) ---

/// Budget for conversational intents.
pub const BUDGET_CONVERSATIONAL: u32 = 20;

/// Budget for status queries.
pub const BUDGET_STATUS_QUERY: u32 = 30;

/// Budget for verification requests.
pub const BUDGET_VERIFICATION_REQUEST: u32 = 40;

/// Budget for information requests.
pub const BUDGET_INFORMATION_REQUEST: u32 = 40;

/// Budget for action requests.
pub const BUDGET_ACTION_REQUEST: u32 = 50;

/// Budget for analysis requests.
pub const BUDGET_ANALYSIS_REQUEST: u32 = 80;

/// Budget for planning assist.
pub const BUDGET_PLANNING_ASSIST: u32 = 80;

/// Budget for generative requests.
pub const BUDGET_GENERATIVE_REQUEST: u32 = 60;

// --- Affect multipliers ---

/// Multiplier for crisis affect (narrow focus).
pub const AFFECT_MULTIPLIER_CRISIS: f64 = 0.5;

/// Multiplier for under-pressure affect (tight focus).
pub const AFFECT_MULTIPLIER_UNDER_PRESSURE: f64 = 0.7;

/// Multiplier for celebratory affect (lighter).
pub const AFFECT_MULTIPLIER_CELEBRATORY: f64 = 0.8;

/// Multiplier for neutral affect (standard).
pub const AFFECT_MULTIPLIER_NEUTRAL: f64 = 1.0;

/// Multiplier for frustrated affect (wider — underlying concern needs context).
pub const AFFECT_MULTIPLIER_FRUSTRATED: f64 = 1.2;

/// Multiplier for exploratory affect (widest — learning needs breadth).
pub const AFFECT_MULTIPLIER_EXPLORATORY: f64 = 1.5;

// --- Scoring bonuses ---

/// Bonus added to significance when an item has memory resonance.
pub const RESONANCE_SIGNIFICANCE_BONUS: f64 = 0.2;

/// Bonus added to significance when an item is temporally urgent.
pub const URGENCY_SIGNIFICANCE_BONUS: f64 = 0.15;
