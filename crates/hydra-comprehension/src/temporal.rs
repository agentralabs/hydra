//! Temporal placement — urgency and horizon classification.
//!
//! Computes urgency from keyword signals and classifies the
//! time horizon of the input. No LLM calls.

use serde::{Deserialize, Serialize};

/// High-urgency keywords and their scores.
const HIGH_URGENCY: &[(&str, f64)] = &[
    ("critical", 0.95),
    ("urgent", 0.9),
    ("emergency", 0.95),
    ("now", 0.8),
    ("immediately", 0.9),
    ("asap", 0.85),
    ("blocking", 0.85),
    ("outage", 0.9),
    ("down", 0.7),
    ("broken", 0.75),
];

/// Low-urgency keywords and their scores.
const LOW_URGENCY: &[(&str, f64)] = &[
    ("plan", 0.2),
    ("eventually", 0.1),
    ("later", 0.15),
    ("someday", 0.1),
    ("backlog", 0.15),
    ("consider", 0.2),
    ("future", 0.15),
    ("maybe", 0.2),
];

/// Time horizon classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Horizon {
    /// Needs immediate action (minutes to hours).
    Immediate,
    /// Short-term action (hours to days).
    ShortTerm,
    /// Medium-term planning (days to weeks).
    MediumTerm,
    /// Long-term or aspirational.
    LongTerm,
}

/// Status of temporal constraints for this input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintStatus {
    /// No temporal constraints detected.
    None,
    /// Input activates a known temporal constraint.
    Activates,
    /// Input may satisfy a temporal constraint.
    Satisfies,
    /// Input may violate a temporal constraint.
    Violates,
}

/// Temporal context derived from input analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalContext {
    /// Urgency score (0.0 = no urgency, 1.0 = maximum urgency).
    pub urgency: f64,
    /// Classified time horizon.
    pub horizon: Horizon,
    /// Whether this input relates to any temporal constraint.
    pub constraint_status: ConstraintStatus,
}

/// Temporal placement engine.
pub struct TemporalPlacement;

impl TemporalPlacement {
    /// Analyze input for temporal signals.
    ///
    /// Computes urgency from keyword matching and classifies the horizon.
    /// Checks for constraint-related keywords.
    pub fn analyze(input: &str) -> TemporalContext {
        let lower = input.to_lowercase();
        let urgency = Self::compute_urgency(&lower);
        let horizon = Self::classify_horizon(urgency);
        let constraint_status = Self::detect_constraint_status(&lower);

        TemporalContext {
            urgency,
            horizon,
            constraint_status,
        }
    }

    /// Compute urgency from keyword signals.
    fn compute_urgency(lower: &str) -> f64 {
        let mut max_high: f64 = 0.0;
        let mut max_low: f64 = 0.0;

        for &(kw, score) in HIGH_URGENCY {
            if lower.contains(kw) && score > max_high {
                max_high = score;
            }
        }

        for &(kw, score) in LOW_URGENCY {
            if lower.contains(kw) && score > max_low {
                max_low = score;
            }
        }

        if max_high > 0.0 && max_low > 0.0 {
            // Conflicting signals: lean toward urgency but dampen.
            max_high * 0.7
        } else if max_high > 0.0 {
            max_high
        } else if max_low > 0.0 {
            max_low
        } else {
            0.5 // Neutral — no temporal signal
        }
    }

    /// Classify time horizon from urgency score.
    fn classify_horizon(urgency: f64) -> Horizon {
        if urgency >= 0.8 {
            Horizon::Immediate
        } else if urgency >= 0.5 {
            Horizon::ShortTerm
        } else if urgency >= 0.25 {
            Horizon::MediumTerm
        } else {
            Horizon::LongTerm
        }
    }

    /// Detect whether input relates to temporal constraints.
    ///
    /// Violates is checked before Activates because "overdue" contains "due".
    fn detect_constraint_status(lower: &str) -> ConstraintStatus {
        if lower.contains("overdue") || lower.contains("missed") || lower.contains("late") {
            ConstraintStatus::Violates
        } else if lower.contains("deadline") || lower.contains("due") || lower.contains("expires") {
            ConstraintStatus::Activates
        } else if lower.contains("completed")
            || lower.contains("finished")
            || lower.contains("done")
        {
            ConstraintStatus::Satisfies
        } else {
            ConstraintStatus::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_urgency_detected() {
        let ctx = TemporalPlacement::analyze("critical outage in production");
        assert!(ctx.urgency >= 0.8);
        assert_eq!(ctx.horizon, Horizon::Immediate);
    }

    #[test]
    fn low_urgency_detected() {
        let ctx = TemporalPlacement::analyze("plan to eventually migrate");
        assert!(ctx.urgency < 0.3);
        assert_eq!(ctx.horizon, Horizon::LongTerm);
    }

    #[test]
    fn neutral_when_no_keywords() {
        let ctx = TemporalPlacement::analyze("write a function that adds numbers");
        assert!((ctx.urgency - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn constraint_activates() {
        let ctx = TemporalPlacement::analyze("the deadline is tomorrow");
        assert_eq!(ctx.constraint_status, ConstraintStatus::Activates);
    }

    #[test]
    fn constraint_violates() {
        let ctx = TemporalPlacement::analyze("this task is overdue");
        assert_eq!(ctx.constraint_status, ConstraintStatus::Violates);
    }
}
