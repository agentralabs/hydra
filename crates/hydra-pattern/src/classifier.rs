//! PatternClassifier -- determine if a situation matches a known pattern.

use crate::{constants::ANTIPATTERN_WARNING_THRESHOLD, entry::PatternEntry};
use serde::{Deserialize, Serialize};

/// A pattern classification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub pattern_id: String,
    pub pattern_name: String,
    pub kind: String,
    pub similarity: f64,
    pub confidence: f64,
    pub response: String,
    pub is_warning: bool,
}

impl ClassificationResult {
    pub fn should_warn(&self) -> bool {
        self.is_warning && self.similarity >= ANTIPATTERN_WARNING_THRESHOLD
    }
}

/// Classify input primitives against a pattern entry.
pub fn classify(entry: &PatternEntry, input_sig: &[String]) -> Option<ClassificationResult> {
    let similarity = entry.similarity_to(input_sig);

    if similarity < crate::constants::PATTERN_MATCH_THRESHOLD {
        return None;
    }

    Some(ClassificationResult {
        pattern_id: entry.id.clone(),
        pattern_name: entry.name.clone(),
        kind: entry.kind.label().to_string(),
        similarity,
        confidence: entry.confidence * similarity,
        response: entry.response.clone(),
        is_warning: entry.kind.is_warning(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{PatternEntry, PatternKind};
    use hydra_axiom::primitives::AxiomPrimitive;

    fn cascade_pattern() -> PatternEntry {
        PatternEntry::new(
            "Cascade Failure",
            "desc",
            PatternKind::AntiPattern {
                failure_mode: "cascade".into(),
                warning_signs: vec![],
            },
            &[AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
            vec!["engineering".into()],
            "Install circuit breakers",
        )
    }

    #[test]
    fn matching_signature_classified() {
        let entry = cascade_pattern();
        let input = vec!["risk".to_string(), "causal-link".to_string()];
        let r = classify(&entry, &input);
        assert!(r.is_some());
        let result = r.expect("should be some");
        assert!(result.similarity >= crate::constants::PATTERN_MATCH_THRESHOLD);
    }

    #[test]
    fn no_match_below_threshold() {
        let entry = cascade_pattern();
        let input = vec!["optimization".to_string()]; // no overlap
        let r = classify(&entry, &input);
        assert!(r.is_none());
    }

    #[test]
    fn antipattern_triggers_warning() {
        let mut entry = cascade_pattern();
        entry.confirm();
        entry.confirm(); // boost confidence
        let input = vec!["risk".to_string(), "causal-link".to_string()];
        let r = classify(&entry, &input).expect("should match");
        assert!(r.is_warning);
        assert!(r.should_warn());
    }
}
