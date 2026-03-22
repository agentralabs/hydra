//! Affect detection — classifies the emotional register of input.
//!
//! Zero LLM calls. Pure keyword density classification.

use crate::constants::{
    AFFECT_STRESS_THRESHOLD, CELEBRATION_KEYWORDS, CRISIS_KEYWORDS, FRUSTRATION_KEYWORDS,
    PRESSURE_KEYWORDS,
};
use serde::{Deserialize, Serialize};

/// The detected interaction register.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionRegister {
    /// No strong emotional signal.
    Neutral,
    /// User is under time or resource pressure.
    UnderPressure,
    /// User is frustrated with repeated issues.
    Frustrated,
    /// Active crisis situation.
    Crisis,
    /// User is celebrating a success.
    Celebratory,
    /// User is in exploratory mode.
    Exploratory,
}

impl InteractionRegister {
    /// Return a human-readable label.
    pub fn label(&self) -> &str {
        match self {
            Self::Neutral => "neutral",
            Self::UnderPressure => "under-pressure",
            Self::Frustrated => "frustrated",
            Self::Crisis => "crisis",
            Self::Celebratory => "celebratory",
            Self::Exploratory => "exploratory",
        }
    }
}

/// The result of affect detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectSignal {
    /// The detected interaction register.
    pub register: InteractionRegister,
    /// Confidence in this classification (0.0 to 1.0).
    pub confidence: f64,
    /// Keywords that triggered the classification.
    pub keywords_detected: Vec<String>,
}

/// Detect the affect (emotional register) of input text.
///
/// Classifies by keyword density. Zero LLM calls.
pub fn detect_affect(text: &str) -> AffectSignal {
    let lower = text.to_lowercase();

    let crisis_hits = count_keywords(&lower, CRISIS_KEYWORDS);
    let pressure_hits = count_keywords(&lower, PRESSURE_KEYWORDS);
    let frustration_hits = count_keywords(&lower, FRUSTRATION_KEYWORDS);
    let celebration_hits = count_keywords(&lower, CELEBRATION_KEYWORDS);

    // Crisis takes priority.
    if crisis_hits.len() >= AFFECT_STRESS_THRESHOLD {
        return make_signal(InteractionRegister::Crisis, 0.9, crisis_hits);
    }

    if pressure_hits.len() >= AFFECT_STRESS_THRESHOLD {
        return make_signal(InteractionRegister::UnderPressure, 0.8, pressure_hits);
    }

    if frustration_hits.len() >= AFFECT_STRESS_THRESHOLD {
        return make_signal(InteractionRegister::Frustrated, 0.8, frustration_hits);
    }

    if celebration_hits.len() >= AFFECT_STRESS_THRESHOLD {
        return make_signal(InteractionRegister::Celebratory, 0.85, celebration_hits);
    }

    // Single-keyword fallbacks with lower confidence.
    if !crisis_hits.is_empty() {
        return make_signal(InteractionRegister::Crisis, 0.6, crisis_hits);
    }
    if !pressure_hits.is_empty() {
        return make_signal(InteractionRegister::UnderPressure, 0.5, pressure_hits);
    }
    if !frustration_hits.is_empty() {
        return make_signal(InteractionRegister::Frustrated, 0.5, frustration_hits);
    }
    if !celebration_hits.is_empty() {
        return make_signal(InteractionRegister::Celebratory, 0.5, celebration_hits);
    }

    make_signal(InteractionRegister::Neutral, 0.7, vec![])
}

/// Count which keywords from a list appear in the text.
fn count_keywords(text: &str, keywords: &[&str]) -> Vec<String> {
    keywords
        .iter()
        .filter(|kw| text.contains(**kw))
        .map(|kw| kw.to_string())
        .collect()
}

/// Construct an AffectSignal.
fn make_signal(
    register: InteractionRegister,
    confidence: f64,
    keywords_detected: Vec<String>,
) -> AffectSignal {
    AffectSignal {
        register,
        confidence,
        keywords_detected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_crisis() {
        let s = detect_affect("the service is broken and the site is down");
        assert_eq!(s.register, InteractionRegister::Crisis);
    }

    #[test]
    fn detects_pressure() {
        let s = detect_affect("this is urgent with a deadline tomorrow");
        assert_eq!(s.register, InteractionRegister::UnderPressure);
    }

    #[test]
    fn detects_frustration() {
        let s = detect_affect("this keeps failing again and again");
        assert_eq!(s.register, InteractionRegister::Frustrated);
    }

    #[test]
    fn detects_celebration() {
        let s = detect_affect("the tests passed and we shipped it, success");
        assert_eq!(s.register, InteractionRegister::Celebratory);
    }

    #[test]
    fn neutral_for_normal() {
        let s = detect_affect("please review the architecture document");
        assert_eq!(s.register, InteractionRegister::Neutral);
    }
}
