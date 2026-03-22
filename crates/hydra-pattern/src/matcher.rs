//! PatternMatcher -- find all patterns matching an input signature.

use crate::{
    classifier::{classify, ClassificationResult},
    entry::PatternEntry,
};

/// Find all patterns that match the given primitive signature.
pub fn find_matches(library: &[PatternEntry], input_sig: &[String]) -> Vec<ClassificationResult> {
    let mut results: Vec<ClassificationResult> = library
        .iter()
        .filter_map(|entry| classify(entry, input_sig))
        .collect();

    // Sort by confidence descending
    results.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

/// Find anti-pattern warnings only.
pub fn find_warnings(library: &[PatternEntry], input_sig: &[String]) -> Vec<ClassificationResult> {
    find_matches(library, input_sig)
        .into_iter()
        .filter(|r| r.should_warn())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{PatternEntry, PatternKind};
    use hydra_axiom::primitives::AxiomPrimitive;

    fn make_library() -> Vec<PatternEntry> {
        let mut cascade = PatternEntry::new(
            "Cascade Failure",
            "desc",
            PatternKind::AntiPattern {
                failure_mode: "cascade".into(),
                warning_signs: vec![],
            },
            &[AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
            vec!["engineering".into()],
            "circuit breakers",
        );
        cascade.confirm();
        cascade.confirm();

        let circuit_breaker = PatternEntry::new(
            "Circuit Breaker",
            "desc",
            PatternKind::SuccessPattern {
                success_condition: "isolated failures".into(),
                key_steps: vec!["detect".into(), "open".into(), "recover".into()],
            },
            &[AxiomPrimitive::Risk, AxiomPrimitive::Constraint],
            vec!["engineering".into(), "finance".into()],
            "implement circuit breaker pattern",
        );
        vec![cascade, circuit_breaker]
    }

    #[test]
    fn both_patterns_match_risk_causal() {
        let library = make_library();
        let input_sig = vec!["risk".into(), "causal-link".into()];
        let matches = find_matches(&library, &input_sig);
        assert!(!matches.is_empty());
    }

    #[test]
    fn only_antipattern_in_warnings() {
        let library = make_library();
        let input_sig = vec!["risk".into(), "causal-link".into()];
        let warnings = find_warnings(&library, &input_sig);
        assert!(warnings.iter().all(|w| w.is_warning));
    }
}
