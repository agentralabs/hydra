//! TaskDecomposer — converts a task description into axiom primitives.

use crate::constants::MAX_DECOMPOSITION_PRIMITIVES;
use hydra_axiom::AxiomPrimitive;
use serde::{Deserialize, Serialize};

/// A decomposed task — description mapped to axiom primitives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDecomposition {
    /// The original task description.
    pub description: String,
    /// Axiom primitives extracted from the task.
    pub primitives: Vec<AxiomPrimitive>,
}

/// Type alias for the keyword-to-primitive mapping entry.
type KeywordMapping = (&'static [&'static str], fn() -> AxiomPrimitive);

/// Keyword-to-primitive mapping table.
///
/// Maps common task keywords to the axiom primitives they imply.
/// This is intentionally broad — the LLM refines in production.
const KEYWORD_MAPPINGS: &[KeywordMapping] = &[
    (&["risk", "danger", "threat", "vulnerability"], || {
        AxiomPrimitive::Risk
    }),
    (&["optimize", "improve", "enhance", "performance"], || {
        AxiomPrimitive::Optimization
    }),
    (&["constraint", "limit", "restrict", "boundary"], || {
        AxiomPrimitive::Constraint
    }),
    (&["probability", "chance", "likelihood", "odds"], || {
        AxiomPrimitive::Probability
    }),
    (&["uncertain", "unknown", "ambiguous", "unclear"], || {
        AxiomPrimitive::Uncertainty
    }),
    (&["cause", "effect", "because", "therefore"], || {
        AxiomPrimitive::CausalLink
    }),
    (&["time", "sequence", "order", "before", "after"], || {
        AxiomPrimitive::TemporalSequence
    }),
    (&["resource", "allocate", "budget", "capacity"], || {
        AxiomPrimitive::ResourceAllocation
    }),
    (&["depend", "require", "need", "prerequisite"], || {
        AxiomPrimitive::Dependency
    }),
    (&["trust", "verify", "authenticate", "credential"], || {
        AxiomPrimitive::TrustRelation
    }),
    (
        &["coordinate", "synchronize", "collaborate", "agent"],
        || AxiomPrimitive::CoordinationEquilibrium,
    ),
    (&["emerge", "pattern", "complex", "system"], || {
        AxiomPrimitive::EmergencePattern
    }),
    (&["adversary", "attack", "defend", "security"], || {
        AxiomPrimitive::AdversarialModel
    }),
    (&["information", "signal", "data", "value"], || {
        AxiomPrimitive::InformationValue
    }),
];

/// Decompose a task description into axiom primitives.
///
/// Extracts keywords from the description and maps them to primitives.
/// Deduplicates and caps at `MAX_DECOMPOSITION_PRIMITIVES`.
pub fn decompose(description: &str) -> TaskDecomposition {
    let words: Vec<String> = description
        .split_whitespace()
        .map(|w| w.to_lowercase().replace(|c: char| !c.is_alphanumeric(), ""))
        .collect();

    let mut primitives: Vec<AxiomPrimitive> = Vec::new();

    for (keywords, constructor) in KEYWORD_MAPPINGS {
        for word in &words {
            let stem = word.strip_suffix('s').unwrap_or(word);
            if keywords.contains(&word.as_str()) || keywords.contains(&stem) {
                let primitive = constructor();
                if !primitives.contains(&primitive) {
                    primitives.push(primitive);
                }
                break;
            }
        }
    }

    primitives.truncate(MAX_DECOMPOSITION_PRIMITIVES);

    TaskDecomposition {
        description: description.to_string(),
        primitives,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompose_risk_task() {
        let td = decompose("assess risk of api deployment");
        assert!(td.primitives.contains(&AxiomPrimitive::Risk));
    }

    #[test]
    fn decompose_multi_primitive() {
        let td = decompose("optimize resource allocation under time constraints");
        assert!(td.primitives.contains(&AxiomPrimitive::Optimization));
        assert!(td.primitives.contains(&AxiomPrimitive::ResourceAllocation));
        assert!(td.primitives.contains(&AxiomPrimitive::Constraint));
    }

    #[test]
    fn decompose_empty() {
        let td = decompose("");
        assert!(td.primitives.is_empty());
    }

    #[test]
    fn decompose_no_duplicates() {
        let td = decompose("risk risk risk risk");
        let risk_count = td
            .primitives
            .iter()
            .filter(|p| **p == AxiomPrimitive::Risk)
            .count();
        assert_eq!(risk_count, 1);
    }

    #[test]
    fn decompose_capped() {
        let td = decompose(
            "risk optimize constraint probability uncertain cause time \
             resource depend trust coordinate emerge adversary information value",
        );
        assert!(td.primitives.len() <= MAX_DECOMPOSITION_PRIMITIVES);
    }
}
