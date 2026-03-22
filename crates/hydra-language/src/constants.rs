//! Constants for hydra-language.
//!
//! All tunable parameters live here. No magic numbers elsewhere.

/// Confidence threshold to classify intent as clear.
pub const INTENT_CLEAR_THRESHOLD: f64 = 0.7;

/// Hedging reduces confidence by this factor.
pub const HEDGE_CONFIDENCE_PENALTY: f64 = 0.15;

/// Urgency stress threshold (affect module).
pub const AFFECT_STRESS_THRESHOLD: usize = 2;

/// Minimum words to attempt deep intent extraction.
pub const MIN_WORDS_FOR_DEEP_INTENT: usize = 4;

/// Depth signal: fraction of surface words before deeper need flagged.
pub const DEPTH_SURFACE_THRESHOLD: f64 = 0.6;

/// Keywords indicating a crisis situation.
pub const CRISIS_KEYWORDS: &[&str] = &["outage", "broken", "down", "affected"];

/// Keywords indicating pressure.
pub const PRESSURE_KEYWORDS: &[&str] = &["urgent", "asap", "deadline", "running out"];

/// Keywords indicating frustration.
pub const FRUSTRATION_KEYWORDS: &[&str] = &["failing", "again", "third time", "keep"];

/// Keywords indicating celebration.
pub const CELEBRATION_KEYWORDS: &[&str] = &["passed", "shipped", "success", "done"];

/// Default response depth when crisis is detected.
pub const RESPONSE_DEPTH_CRISIS_OVERRIDE: &str = "Brief";
