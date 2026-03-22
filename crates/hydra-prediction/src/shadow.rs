//! Shadow execution — running predictions without side effects.

use serde::{Deserialize, Serialize};

/// The outcome of a shadow (predicted) execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowOutcome {
    /// Description of the predicted outcome.
    pub description: String,
    /// Confidence in this outcome.
    pub confidence: f64,
    /// Key-value pairs of predicted state changes.
    pub state_changes: Vec<(String, String)>,
}

/// The outcome that actually occurred.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualOutcome {
    /// Description of what actually happened.
    pub description: String,
    /// Key-value pairs of actual state changes.
    pub state_changes: Vec<(String, String)>,
}

/// Divergence between a shadow and actual outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeDivergence {
    /// The divergence score (0.0 = perfect match, 1.0 = complete mismatch).
    pub score: f64,
    /// Which state changes diverged.
    pub diverged_keys: Vec<String>,
    /// Summary of the divergence.
    pub summary: String,
}

/// Compute the divergence between a shadow and actual outcome.
///
/// Compares state change keys and values. The score is the fraction
/// of keys that differ between predicted and actual.
pub fn compute_divergence(shadow: &ShadowOutcome, actual: &ActualOutcome) -> OutcomeDivergence {
    if shadow.state_changes.is_empty() && actual.state_changes.is_empty() {
        return OutcomeDivergence {
            score: 0.0,
            diverged_keys: vec![],
            summary: "both empty — no divergence".to_string(),
        };
    }

    let all_keys: Vec<String> = shadow
        .state_changes
        .iter()
        .map(|(k, _)| k.clone())
        .chain(actual.state_changes.iter().map(|(k, _)| k.clone()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let total = all_keys.len();
    let mut diverged_keys = Vec::new();

    for key in &all_keys {
        let shadow_val = shadow
            .state_changes
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str());
        let actual_val = actual
            .state_changes
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str());

        if shadow_val != actual_val {
            diverged_keys.push(key.clone());
        }
    }

    let score = if total > 0 {
        diverged_keys.len() as f64 / total as f64
    } else {
        0.0
    };

    let summary = format!(
        "{} of {} state changes diverged",
        diverged_keys.len(),
        total
    );

    OutcomeDivergence {
        score,
        diverged_keys,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_divergence_on_match() {
        let shadow = ShadowOutcome {
            description: "test".into(),
            confidence: 0.8,
            state_changes: vec![("key".into(), "val".into())],
        };
        let actual = ActualOutcome {
            description: "test".into(),
            state_changes: vec![("key".into(), "val".into())],
        };
        let div = compute_divergence(&shadow, &actual);
        assert!((div.score).abs() < f64::EPSILON);
    }

    #[test]
    fn full_divergence_on_mismatch() {
        let shadow = ShadowOutcome {
            description: "test".into(),
            confidence: 0.8,
            state_changes: vec![("key".into(), "a".into())],
        };
        let actual = ActualOutcome {
            description: "test".into(),
            state_changes: vec![("key".into(), "b".into())],
        };
        let div = compute_divergence(&shadow, &actual);
        assert!((div.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_outcomes_no_divergence() {
        let shadow = ShadowOutcome {
            description: "test".into(),
            confidence: 0.5,
            state_changes: vec![],
        };
        let actual = ActualOutcome {
            description: "test".into(),
            state_changes: vec![],
        };
        let div = compute_divergence(&shadow, &actual);
        assert!((div.score).abs() < f64::EPSILON);
    }
}
