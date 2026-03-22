//! Cross-domain matcher — finds structural similarities across domains.

use crate::constants::SYNTHESIS_SIMILARITY_THRESHOLD;
use crate::pattern::StructuralPattern;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A cross-domain match between two structural patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainMatch {
    /// The first domain in the match.
    pub domain_a: String,
    /// The second domain in the match.
    pub domain_b: String,
    /// Primitive type labels shared between the two patterns.
    pub shared_primitives: Vec<String>,
    /// Jaccard similarity between the two patterns.
    pub similarity: f64,
}

impl CrossDomainMatch {
    /// Return a one-line summary for display.
    pub fn summary(&self) -> String {
        format!(
            "match[{}<->{}] shared=[{}] sim={:.2}",
            self.domain_a,
            self.domain_b,
            self.shared_primitives.join(", "),
            self.similarity,
        )
    }
}

/// Find cross-domain matches among a set of structural patterns.
///
/// Only matches between DIFFERENT domains that exceed the similarity
/// threshold are returned. Patterns within the same domain are skipped.
pub fn find_cross_domain_matches(patterns: &[StructuralPattern]) -> Vec<CrossDomainMatch> {
    let mut matches = Vec::new();

    for i in 0..patterns.len() {
        for j in (i + 1)..patterns.len() {
            let a = &patterns[i];
            let b = &patterns[j];

            // Only match across different domains.
            if a.domain == b.domain {
                continue;
            }

            let sim = a.similarity(b);
            if sim < SYNTHESIS_SIMILARITY_THRESHOLD {
                continue;
            }

            let set_a: HashSet<&str> = a.primitive_types.iter().map(|s| s.as_str()).collect();
            let set_b: HashSet<&str> = b.primitive_types.iter().map(|s| s.as_str()).collect();
            let shared: Vec<String> = set_a
                .intersection(&set_b)
                .map(|s| (*s).to_string())
                .collect();

            matches.push(CrossDomainMatch {
                domain_a: a.domain.clone(),
                domain_b: b.domain.clone(),
                shared_primitives: shared,
                similarity: sim,
            });
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_axiom::AxiomPrimitive;

    #[test]
    fn finds_cross_domain_match() {
        let patterns = vec![
            StructuralPattern::from_primitives(
                "engineering",
                &[AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
                "eng pattern",
            ),
            StructuralPattern::from_primitives(
                "finance",
                &[AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
                "fin pattern",
            ),
        ];
        let matches = find_cross_domain_matches(&patterns);
        assert_eq!(matches.len(), 1);
        assert!((matches[0].similarity - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn same_domain_skipped() {
        let patterns = vec![
            StructuralPattern::from_primitives("engineering", &[AxiomPrimitive::Risk], "pattern a"),
            StructuralPattern::from_primitives("engineering", &[AxiomPrimitive::Risk], "pattern b"),
        ];
        let matches = find_cross_domain_matches(&patterns);
        assert!(matches.is_empty());
    }

    #[test]
    fn below_threshold_excluded() {
        let patterns = vec![
            StructuralPattern::from_primitives("engineering", &[AxiomPrimitive::Risk], "eng"),
            StructuralPattern::from_primitives("finance", &[AxiomPrimitive::Optimization], "fin"),
        ];
        let matches = find_cross_domain_matches(&patterns);
        assert!(matches.is_empty());
    }
}
