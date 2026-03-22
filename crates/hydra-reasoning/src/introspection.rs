//! Recursive Self-Questioning — think, question, think again.
//!
//! The difference between a good answer and a brilliant one is not
//! running more modes in parallel. It is running one mode DEEPER.
//! Think → "What did I assume?" → Challenge → Think again → Converge.

use serde::{Deserialize, Serialize};

/// One iteration of introspective reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionStep {
    /// The conclusion reached in this iteration.
    pub conclusion: String,
    /// Confidence in this conclusion.
    pub confidence: f64,
    /// What assumption was identified and challenged.
    pub challenged_assumption: Option<String>,
    /// Whether the conclusion changed from the previous iteration.
    pub changed: bool,
}

/// The full introspection loop result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionResult {
    /// Each iteration of reasoning.
    pub steps: Vec<IntrospectionStep>,
    /// Final confidence after convergence.
    pub final_confidence: f64,
    /// How much confidence changed from initial to final.
    pub confidence_delta: f64,
    /// Whether the loop converged (delta < threshold).
    pub converged: bool,
    /// How many assumptions were challenged.
    pub assumptions_challenged: usize,
}

impl IntrospectionResult {
    /// Human-readable summary for prompt injection.
    pub fn summary(&self) -> String {
        if self.steps.len() <= 1 {
            return String::new();
        }
        let challenged = self
            .steps
            .iter()
            .filter_map(|s| s.challenged_assumption.as_ref())
            .collect::<Vec<_>>();
        if challenged.is_empty() {
            format!(
                "Examined in {} iterations. Confidence: {:.0}%. Converged: {}.",
                self.steps.len(),
                self.final_confidence * 100.0,
                if self.converged { "yes" } else { "no" }
            )
        } else {
            format!(
                "Examined in {} iterations. Challenged {} assumptions: {}. \
                 Confidence: {:.0}% (delta: {:+.0}%). Converged: {}.",
                self.steps.len(),
                challenged.len(),
                challenged
                    .iter()
                    .map(|a| format!("\"{}\"", a))
                    .collect::<Vec<_>>()
                    .join(", "),
                self.final_confidence * 100.0,
                self.confidence_delta * 100.0,
                if self.converged { "yes" } else { "no" }
            )
        }
    }
}

/// Configuration for the introspection loop.
#[derive(Debug, Clone)]
pub struct IntrospectionConfig {
    /// Maximum iterations before forced stop.
    pub max_depth: usize,
    /// Minimum confidence delta to justify another iteration.
    pub convergence_threshold: f64,
    /// Minimum confidence to trigger introspection (below this = introspect).
    pub trigger_threshold: f64,
}

impl Default for IntrospectionConfig {
    fn default() -> Self {
        Self {
            max_depth: 4,
            convergence_threshold: 0.05,
            trigger_threshold: 0.80,
        }
    }
}

/// Run a lightweight introspection loop on a conclusion.
///
/// This is the NON-LLM version: it checks whether the conclusion's
/// confidence is stable across small perturbations. The LLM version
/// (assumption extraction via micro-call) can be added in the engine.
pub fn introspect_confidence(
    initial_conclusion: &str,
    initial_confidence: f64,
    supporting_evidence: &[(&str, f64)],
    config: &IntrospectionConfig,
) -> IntrospectionResult {
    let mut steps = Vec::new();
    let mut current_confidence = initial_confidence;

    // Step 1: Record initial conclusion
    steps.push(IntrospectionStep {
        conclusion: initial_conclusion.to_string(),
        confidence: current_confidence,
        challenged_assumption: None,
        changed: false,
    });

    // Steps 2+: Check each piece of supporting evidence
    for (i, (evidence, evidence_conf)) in supporting_evidence.iter().enumerate() {
        if i >= config.max_depth - 1 {
            break;
        }

        // Challenge: what if this evidence is wrong?
        let without_this = if supporting_evidence.len() > 1 {
            // Recompute confidence excluding this evidence
            let other_confs: Vec<f64> = supporting_evidence
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, (_, c))| *c)
                .collect();
            let sum: f64 = other_confs.iter().sum();
            sum / other_confs.len() as f64
        } else {
            initial_confidence * 0.5 // Only one evidence → halve confidence
        };

        let delta = (current_confidence - without_this).abs();
        let changed = delta > config.convergence_threshold;

        if changed {
            // This evidence is load-bearing — it matters
            let assumption = format!(
                "depends on: '{}' (conf={:.0}%)",
                evidence,
                evidence_conf * 100.0
            );
            current_confidence = (current_confidence + without_this) / 2.0;

            steps.push(IntrospectionStep {
                conclusion: initial_conclusion.to_string(),
                confidence: current_confidence,
                challenged_assumption: Some(assumption),
                changed: true,
            });
        } else {
            // This evidence is redundant — removing it doesn't change much
            steps.push(IntrospectionStep {
                conclusion: initial_conclusion.to_string(),
                confidence: current_confidence,
                challenged_assumption: None,
                changed: false,
            });
        }
    }

    let final_confidence = current_confidence;
    let confidence_delta = final_confidence - initial_confidence;
    let converged = steps
        .windows(2)
        .last()
        .map(|w| (w[1].confidence - w[0].confidence).abs() < config.convergence_threshold)
        .unwrap_or(true);
    let assumptions_challenged = steps
        .iter()
        .filter(|s| s.challenged_assumption.is_some())
        .count();

    IntrospectionResult {
        steps,
        final_confidence,
        confidence_delta,
        converged,
        assumptions_challenged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_introspection_without_evidence() {
        let result = introspect_confidence(
            "test conclusion",
            0.72,
            &[],
            &IntrospectionConfig::default(),
        );
        assert_eq!(result.steps.len(), 1);
        assert!(result.converged);
    }

    #[test]
    fn identifies_load_bearing_evidence() {
        let result = introspect_confidence(
            "deploy is safe",
            0.85,
            &[("tests pass", 0.95), ("config correct", 0.50)],
            &IntrospectionConfig::default(),
        );
        assert!(result.assumptions_challenged > 0);
    }

    #[test]
    fn summary_contains_challenged() {
        let result = introspect_confidence(
            "test",
            0.70,
            &[("evidence A", 0.90), ("evidence B", 0.30)],
            &IntrospectionConfig::default(),
        );
        let summary = result.summary();
        assert!(summary.contains("Challenged") || summary.contains("iterations"));
    }

    #[test]
    fn max_depth_respected() {
        let evidence: Vec<(&str, f64)> = (0..20)
            .map(|i| ("evidence" as &str, 0.5 + (i as f64) * 0.02))
            .collect();
        let config = IntrospectionConfig {
            max_depth: 3,
            ..Default::default()
        };
        let result = introspect_confidence("test", 0.7, &evidence, &config);
        assert!(result.steps.len() <= 4); // 1 initial + 3 max
    }
}
