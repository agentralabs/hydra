//! PatternEntry -- one pattern at the axiom level.
//! Domain-agnostic. Expressed in axiom primitives.
//! The same pattern can appear in finance, engineering, security.

use hydra_axiom::primitives::AxiomPrimitive;
use serde::{Deserialize, Serialize};

/// Whether a pattern is an anti-pattern or a success pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternKind {
    /// A pattern that leads to failure if not interrupted.
    AntiPattern {
        failure_mode: String,
        warning_signs: Vec<String>,
    },
    /// A pattern that consistently leads to success.
    SuccessPattern {
        success_condition: String,
        key_steps: Vec<String>,
    },
}

impl PatternKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AntiPattern { .. } => "anti-pattern",
            Self::SuccessPattern { .. } => "success-pattern",
        }
    }

    pub fn is_warning(&self) -> bool {
        matches!(self, Self::AntiPattern { .. })
    }
}

/// One entry in the pattern library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: PatternKind,
    /// Axiom primitive signature -- domain-agnostic fingerprint.
    pub signature: Vec<String>,
    /// Domains where this pattern has been observed.
    pub domains: Vec<String>,
    /// How many times this pattern has been confirmed.
    pub observations: usize,
    /// Confidence in this pattern (from observations).
    pub confidence: f64,
    /// The recommended response when this pattern is detected.
    pub response: String,
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

impl PatternEntry {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        kind: PatternKind,
        primitives: &[AxiomPrimitive],
        domains: Vec<String>,
        response: impl Into<String>,
    ) -> Self {
        let signature: Vec<String> = {
            let mut sigs: Vec<String> = primitives.iter().map(|p| p.label().to_string()).collect();
            sigs.sort();
            sigs.dedup();
            sigs
        };

        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            kind,
            signature,
            domains,
            observations: 1,
            confidence: 0.6,
            response: response.into(),
            added_at: now,
            last_seen: now,
        }
    }

    /// Update when this pattern is confirmed again.
    pub fn confirm(&mut self) {
        self.observations += 1;
        // Confidence increases with observations, asymptotically approaching 1.0
        self.confidence = 1.0 - (1.0 / (self.observations as f64 + 1.0));
        self.last_seen = chrono::Utc::now();
    }

    pub fn is_proven(&self) -> bool {
        self.observations >= crate::constants::MIN_PROVEN_OBSERVATIONS
    }

    /// Similarity to a set of input primitives (Jaccard on signature).
    pub fn similarity_to(&self, input_sig: &[String]) -> f64 {
        if self.signature.is_empty() && input_sig.is_empty() {
            return 1.0;
        }
        let a: std::collections::HashSet<&str> =
            self.signature.iter().map(|s| s.as_str()).collect();
        let b: std::collections::HashSet<&str> = input_sig.iter().map(|s| s.as_str()).collect();
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let inter = a.intersection(&b).count();
        let union = a.union(&b).count();
        inter as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry() -> PatternEntry {
        PatternEntry::new(
            "Cascade Failure",
            "Small correlated failures that compound into system-wide failure",
            PatternKind::AntiPattern {
                failure_mode: "system-wide cascade".into(),
                warning_signs: vec!["correlated errors".into(), "increasing latency".into()],
            },
            &[
                AxiomPrimitive::Risk,
                AxiomPrimitive::CausalLink,
                AxiomPrimitive::Dependency,
            ],
            vec!["engineering".into(), "finance".into()],
            "Install circuit breakers at dependency boundaries",
        )
    }

    #[test]
    fn pattern_has_signature() {
        let e = make_entry();
        assert!(!e.signature.is_empty());
        assert!(e.signature.contains(&"risk".to_string()));
    }

    #[test]
    fn confirmation_increases_confidence() {
        let mut e = make_entry();
        let before = e.confidence;
        e.confirm();
        assert!(e.confidence > before);
        assert_eq!(e.observations, 2);
    }

    #[test]
    fn similarity_identical_signatures() {
        let e = make_entry();
        let input = e.signature.clone();
        assert!((e.similarity_to(&input) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn similarity_zero_disjoint() {
        let e = make_entry();
        let input = vec!["optimization".to_string()];
        // No overlap with risk+causal-link+dependency
        let sim = e.similarity_to(&input);
        assert_eq!(sim, 0.0);
    }
}
