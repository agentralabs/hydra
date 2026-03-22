//! Settlement constants — all configurable limits and thresholds.
//!
//! No magic numbers in business logic. Every constant lives here.

/// Token cost per 1K tokens (in cost units, not USD — portable).
pub const COST_PER_1K_TOKENS: f64 = 1.0;

/// Attention cost per FOCUS_UNIT consumed.
pub const COST_PER_FOCUS_UNIT: f64 = 0.1;

/// Time cost per second of wall-clock execution.
pub const COST_PER_SECOND: f64 = 0.01;

/// Overhead multiplier for each rerouting attempt.
pub const REROUTING_OVERHEAD_MULTIPLIER: f64 = 0.15;

/// Sister call base cost (per call, regardless of tokens).
pub const SISTER_CALL_BASE_COST: f64 = 0.5;

/// Maximum settlement records stored in memory.
pub const MAX_SETTLEMENT_RECORDS: usize = 1_000_000;

/// Maximum records in one settlement period query.
pub const MAX_PERIOD_RECORDS: usize = 10_000;

/// Settlement record integrity hash label.
pub const SETTLEMENT_HASH_LABEL: &str = "sha256-settlement";
