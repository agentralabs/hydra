//! Synthesis insight — cross-domain knowledge transfer suggestions.

use crate::matcher::CrossDomainMatch;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A synthesis insight suggesting knowledge transfer between domains.
///
/// Generated entirely from axiom primitives and structural pattern matching.
/// No LLM calls are made — all narratives are built from primitive labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisInsight {
    /// Unique identifier for this insight.
    pub id: String,
    /// The first domain in the insight.
    pub domain_a: String,
    /// The second domain in the insight.
    pub domain_b: String,
    /// Shared primitive type labels between the domains.
    pub shared: Vec<String>,
    /// Confidence in this insight (0.0 to 1.0).
    pub confidence: f64,
    /// Human-readable narrative explaining the connection.
    pub narrative: String,
    /// Suggested transfer hint for the user.
    pub transfer_hint: String,
    /// When this insight was generated.
    pub timestamp: DateTime<Utc>,
}

impl SynthesisInsight {
    /// Create an insight from a cross-domain match.
    pub fn from_match(cdm: &CrossDomainMatch) -> Self {
        let narrative = build_narrative(&cdm.domain_a, &cdm.domain_b, &cdm.shared_primitives);
        let transfer_hint =
            build_transfer_hint(&cdm.domain_a, &cdm.domain_b, &cdm.shared_primitives);

        Self {
            id: Uuid::new_v4().to_string(),
            domain_a: cdm.domain_a.clone(),
            domain_b: cdm.domain_b.clone(),
            shared: cdm.shared_primitives.clone(),
            confidence: cdm.similarity,
            narrative,
            transfer_hint,
            timestamp: Utc::now(),
        }
    }

    /// Return a one-line summary for display.
    pub fn summary(&self) -> String {
        format!(
            "insight[{}] {}<->{} shared={} conf={:.2}",
            &self.id[..8],
            self.domain_a,
            self.domain_b,
            self.shared.len(),
            self.confidence,
        )
    }
}

/// Build a human-readable narrative from shared primitives.
fn build_narrative(domain_a: &str, domain_b: &str, shared: &[String]) -> String {
    if shared.is_empty() {
        return format!(
            "{} and {} share no common structural primitives",
            domain_a, domain_b,
        );
    }
    let primitives_text = shared.join(", ");
    format!(
        "{} and {} share structural primitives [{}], suggesting analogous reasoning patterns",
        domain_a, domain_b, primitives_text,
    )
}

/// Build a transfer hint suggesting what to try across domains.
fn build_transfer_hint(domain_a: &str, domain_b: &str, shared: &[String]) -> String {
    if shared.is_empty() {
        return "no transfer opportunity identified".to_string();
    }
    let first_primitive = &shared[0];
    format!(
        "strategies addressing {} in {} may apply to {} given shared {} structure",
        first_primitive, domain_a, domain_b, first_primitive,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::CrossDomainMatch;

    fn make_match() -> CrossDomainMatch {
        CrossDomainMatch {
            domain_a: "engineering".to_string(),
            domain_b: "finance".to_string(),
            shared_primitives: vec!["risk".to_string(), "causal-link".to_string()],
            similarity: 0.85,
        }
    }

    #[test]
    fn insight_from_match() {
        let cdm = make_match();
        let insight = SynthesisInsight::from_match(&cdm);
        assert_eq!(insight.domain_a, "engineering");
        assert_eq!(insight.domain_b, "finance");
        assert!(!insight.narrative.is_empty());
        assert!(!insight.transfer_hint.is_empty());
    }

    #[test]
    fn narrative_contains_domains() {
        let cdm = make_match();
        let insight = SynthesisInsight::from_match(&cdm);
        assert!(insight.narrative.contains("engineering"));
        assert!(insight.narrative.contains("finance"));
    }

    #[test]
    fn summary_format() {
        let cdm = make_match();
        let insight = SynthesisInsight::from_match(&cdm);
        let s = insight.summary();
        assert!(s.contains("insight["));
        assert!(s.contains("engineering"));
        assert!(s.contains("finance"));
    }

    #[test]
    fn empty_shared_narrative() {
        let cdm = CrossDomainMatch {
            domain_a: "a".to_string(),
            domain_b: "b".to_string(),
            shared_primitives: vec![],
            similarity: 0.0,
        };
        let insight = SynthesisInsight::from_match(&cdm);
        assert!(insight.narrative.contains("no common structural"));
    }
}
