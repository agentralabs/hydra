//! Primitive mapping — extracts axiom primitives from input text.
//!
//! Maps recognized keywords to `AxiomPrimitive` variants. No LLM calls.

use crate::constants::MAX_PRIMITIVES_PER_INPUT;
use hydra_axiom::AxiomPrimitive;
use std::collections::HashSet;

/// Keyword-to-primitive mapping entry.
struct PrimitiveRule {
    /// Keywords that trigger this primitive.
    keywords: &'static [&'static str],
    /// The primitive to emit.
    primitive: AxiomPrimitive,
}

/// Builds the static rule table for keyword-to-primitive mapping.
fn rules() -> Vec<PrimitiveRule> {
    vec![
        PrimitiveRule {
            keywords: &["risk", "vulnerability", "threat", "danger", "hazard"],
            primitive: AxiomPrimitive::Risk,
        },
        PrimitiveRule {
            keywords: &["constraint", "limit", "budget", "cap", "bound"],
            primitive: AxiomPrimitive::Constraint,
        },
        PrimitiveRule {
            keywords: &["deploy", "build", "execute", "run", "launch", "trigger"],
            primitive: AxiomPrimitive::CausalLink,
        },
        PrimitiveRule {
            keywords: &["optimize", "efficiency", "performance", "improve", "speed"],
            primitive: AxiomPrimitive::Optimization,
        },
        PrimitiveRule {
            keywords: &["depend", "require", "need", "prerequisite", "import"],
            primitive: AxiomPrimitive::Dependency,
        },
        PrimitiveRule {
            keywords: &["uncertainty", "unknown", "unclear", "ambiguous", "maybe"],
            primitive: AxiomPrimitive::Uncertainty,
        },
        PrimitiveRule {
            keywords: &["trust", "auth", "credential", "permission", "identity"],
            primitive: AxiomPrimitive::TrustRelation,
        },
        PrimitiveRule {
            keywords: &[
                "sequence", "order", "timeline", "before", "after", "schedule",
            ],
            primitive: AxiomPrimitive::TemporalSequence,
        },
    ]
}

/// Maps input text to axiom primitives via keyword matching.
pub struct PrimitiveMapping;

impl PrimitiveMapping {
    /// Extract axiom primitives from input text.
    ///
    /// Scans for keywords and returns up to `MAX_PRIMITIVES_PER_INPUT`
    /// distinct primitives. Order follows the rule table priority.
    pub fn extract(input: &str) -> Vec<AxiomPrimitive> {
        let lower = input.to_lowercase();
        let words: HashSet<&str> = lower.split_whitespace().collect();

        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for rule in &rules() {
            if result.len() >= MAX_PRIMITIVES_PER_INPUT {
                break;
            }
            let matched = rule
                .keywords
                .iter()
                .any(|kw| words.contains(kw) || lower.contains(kw));
            if matched && seen.insert(rule.primitive.label()) {
                result.push(rule.primitive.clone());
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_risk() {
        let prims = PrimitiveMapping::extract("there is a risk of failure");
        assert!(prims.contains(&AxiomPrimitive::Risk));
    }

    #[test]
    fn extracts_constraint() {
        let prims = PrimitiveMapping::extract("we have a budget constraint");
        assert!(prims.contains(&AxiomPrimitive::Constraint));
    }

    #[test]
    fn extracts_multiple() {
        let prims = PrimitiveMapping::extract(
            "deploy the service and optimize performance under budget constraint",
        );
        assert!(prims.len() >= 3);
    }

    #[test]
    fn capped_at_max() {
        let prims = PrimitiveMapping::extract(
            "risk constraint deploy optimize depend uncertainty trust sequence",
        );
        assert!(prims.len() <= MAX_PRIMITIVES_PER_INPUT);
    }

    #[test]
    fn empty_input_returns_empty() {
        let prims = PrimitiveMapping::extract("");
        assert!(prims.is_empty());
    }
}
