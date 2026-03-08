//! ShadowValidator — validate actions by running in shadow first.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::divergence::{DivergenceDetector, DivergenceSeverity};
use super::executor::ShadowExecutor;

/// Outcome of shadow validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationOutcome {
    pub validated: bool,
    pub safe: bool,
    pub divergence_count: usize,
    pub critical_divergences: usize,
    pub recommendation: Recommendation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recommendation {
    Proceed,
    ProceedWithCaution,
    Abort,
}

/// Validates actions through shadow execution
pub struct ShadowValidator {
    executor: ShadowExecutor,
    max_acceptable_divergences: usize,
}

impl ShadowValidator {
    pub fn new() -> Self {
        Self {
            executor: ShadowExecutor::default(),
            max_acceptable_divergences: 3,
        }
    }

    /// Validate an action by running it in shadow first
    pub fn validate(
        &self,
        description: &str,
        input: serde_json::Value,
        expected_outputs: &HashMap<String, serde_json::Value>,
    ) -> ValidationOutcome {
        // Run in shadow
        let run = self.executor.execute(description, input);
        let result = self.executor.result(&run.id);

        match result {
            Some(shadow_result) => {
                let divergences = DivergenceDetector::detect(
                    expected_outputs,
                    &shadow_result.outputs,
                    true,
                    shadow_result.success,
                    true,
                    shadow_result.safe,
                );

                let critical = divergences
                    .iter()
                    .filter(|d| d.severity == DivergenceSeverity::Critical)
                    .count();

                let recommendation = if critical > 0 {
                    Recommendation::Abort
                } else if divergences.len() > self.max_acceptable_divergences {
                    Recommendation::ProceedWithCaution
                } else {
                    Recommendation::Proceed
                };

                ValidationOutcome {
                    validated: true,
                    safe: shadow_result.safe && critical == 0,
                    divergence_count: divergences.len(),
                    critical_divergences: critical,
                    recommendation,
                }
            }
            None => ValidationOutcome {
                validated: false,
                safe: false,
                divergence_count: 0,
                critical_divergences: 0,
                recommendation: Recommendation::ProceedWithCaution,
            },
        }
    }
}

impl Default for ShadowValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_validation() {
        let validator = ShadowValidator::new();
        let expected = HashMap::from([("shadow_output".into(), serde_json::json!({"test": true}))]);

        let outcome =
            validator.validate("test action", serde_json::json!({"test": true}), &expected);
        assert!(outcome.validated);
        assert!(outcome.safe);
        assert_eq!(outcome.recommendation, Recommendation::Proceed);
    }

    #[test]
    fn test_safety_check() {
        let validator = ShadowValidator::new();
        // Shadow will succeed, expected matches shadow output
        let expected = HashMap::from([("shadow_output".into(), serde_json::json!("safe_input"))]);

        let outcome = validator.validate("safe action", serde_json::json!("safe_input"), &expected);
        assert!(outcome.safe);
    }
}
