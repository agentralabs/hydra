//! WisdomStatement — the synthesized judgment from all Layer 4 intelligence.

use crate::constants::*;
use crate::input::WisdomInput;
use serde::{Deserialize, Serialize};

/// The recommendation in a wisdom statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Recommendation {
    /// Proceed — confidence above threshold, no critical warnings.
    Proceed,
    /// Proceed with specific conditions met first.
    ProceedWithConditions { conditions: Vec<String> },
    /// Pause and verify — insufficient confidence or significant uncertainty.
    PauseAndVerify { questions: Vec<String> },
    /// Do not proceed — critical pattern or red team finding.
    DoNotProceed { reasons: Vec<String> },
}

impl Recommendation {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Proceed => "PROCEED",
            Self::ProceedWithConditions { .. } => "PROCEED-WITH-CONDITIONS",
            Self::PauseAndVerify { .. } => "PAUSE-AND-VERIFY",
            Self::DoNotProceed { .. } => "DO-NOT-PROCEED",
        }
    }
    pub fn is_proceed(&self) -> bool {
        matches!(self, Self::Proceed | Self::ProceedWithConditions { .. })
    }
}

/// The synthesized wisdom statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WisdomStatement {
    pub id: String,
    pub context: String,
    pub recommendation: Recommendation,
    pub confidence: f64,
    pub reasoning_chain: Vec<String>,
    pub key_uncertainties: Vec<String>,
    /// What would change this recommendation?
    pub reversal_conditions: Vec<String>,
    pub is_uncertain: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl WisdomStatement {
    /// Synthesize a wisdom statement from all Layer 4 inputs.
    pub fn synthesize(input: &WisdomInput) -> Self {
        let mut reasoning = Vec::new();
        let mut conditions = Vec::new();
        let questions = Vec::new();
        let mut reasons = Vec::new();
        let mut reversals = Vec::new();
        let mut uncertainties = Vec::new();

        // Start from base confidence
        let mut confidence = input.base_confidence;

        // Apply calibration if available
        if let Some(cal) = &input.calibration {
            confidence = cal.calibrated_confidence;
            if cal.is_reliable {
                reasoning.push(format!(
                    "Calibration applied: {} bias in {} domain ({:+.2} offset). \
                     Adjusted confidence from {:.2} to {:.2}.",
                    cal.bias_direction,
                    input.domain,
                    cal.calibrated_confidence - cal.raw_confidence,
                    cal.raw_confidence,
                    cal.calibrated_confidence,
                ));
            } else {
                uncertainties.push(format!(
                    "Insufficient calibration data for {} domain \
                     — using uncalibrated confidence.",
                    input.domain
                ));
            }
        }

        // Pattern evidence
        let warning_patterns: Vec<_> = input.patterns.iter().filter(|p| p.is_warning).collect();
        if !warning_patterns.is_empty() {
            for p in &warning_patterns {
                reasoning.push(format!(
                    "Pattern match: '{}' detected (similarity: {:.0}%). Response: {}",
                    p.pattern_name,
                    p.similarity * 100.0,
                    p.response,
                ));
                conditions.push(p.response.clone());
                confidence *= 1.0 - p.similarity * 0.15;
            }
            reversals.push(format!(
                "If pattern conditions are mitigated ({}), confidence improves.",
                warning_patterns
                    .iter()
                    .map(|p| p.pattern_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Oracle scenarios
        let adverse_scenarios: Vec<_> = input.oracle.iter().filter(|o| o.is_adverse).collect();
        if !adverse_scenarios.is_empty() {
            let total_adverse: f64 = adverse_scenarios.iter().map(|o| o.probability).sum();
            reasoning.push(format!(
                "Oracle: {:.0}% aggregate probability of adverse outcomes \
                 across {} scenario(s).",
                total_adverse * 100.0,
                adverse_scenarios.len(),
            ));
            if total_adverse > 0.4 {
                confidence *= 1.0 - total_adverse * 0.2;
                for s in &adverse_scenarios {
                    if let Some(intervention) = &s.intervention {
                        conditions.push(intervention.clone());
                    }
                }
            }
            reversals.push(
                "If adverse scenario probability drops below 30%, \
                 proceed without conditions."
                    .to_string(),
            );
        }

        // Red team findings
        let critical_threats: Vec<_> = input
            .redteam
            .iter()
            .filter(|r| r.risk_score >= 0.85)
            .collect();
        let high_threats: Vec<_> = input
            .redteam
            .iter()
            .filter(|r| r.risk_score >= 0.65 && r.risk_score < 0.85)
            .collect();

        if !critical_threats.is_empty() {
            for t in &critical_threats {
                reasons.push(format!(
                    "Critical threat: {} — {}",
                    t.threat_name, t.mitigation
                ));
                reasoning.push(format!(
                    "Red team: CRITICAL threat '{}' identified \
                     (risk score: {:.2}). {}",
                    t.threat_name, t.risk_score, t.mitigation,
                ));
            }
        } else if !high_threats.is_empty() {
            for t in &high_threats {
                conditions.push(format!("Mitigate: {}", t.mitigation));
                reasoning.push(format!(
                    "Red team: HIGH threat '{}' identified \
                     (risk score: {:.2}). {}",
                    t.threat_name, t.risk_score, t.mitigation,
                ));
            }
        }

        // Determine recommendation
        let recommendation = if !reasons.is_empty() {
            Recommendation::DoNotProceed { reasons }
        } else if confidence < MIN_RECOMMENDATION_CONFIDENCE || !questions.is_empty() {
            Recommendation::PauseAndVerify { questions }
        } else if !conditions.is_empty() {
            Recommendation::ProceedWithConditions { conditions }
        } else {
            Recommendation::Proceed
        };

        confidence = confidence.clamp(0.0, 1.0);
        let is_uncertain = confidence < UNCERTAINTY_FLAG_THRESHOLD;

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            context: input.context.clone(),
            recommendation,
            confidence,
            reasoning_chain: reasoning,
            key_uncertainties: uncertainties,
            reversal_conditions: reversals,
            is_uncertain,
            created_at: chrono::Utc::now(),
        }
    }

    /// One-line summary for TUI.
    pub fn tui_summary(&self) -> String {
        format!(
            "[{}] conf={:.2}{} — {} reasoning step(s)",
            self.recommendation.label(),
            self.confidence,
            if self.is_uncertain {
                " [uncertain]"
            } else {
                ""
            },
            self.reasoning_chain.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::*;

    #[test]
    fn proceed_when_no_adverse_signals() {
        let input =
            WisdomInput::new("optimize database query", "engineering").with_base_confidence(0.82);
        let stmt = WisdomStatement::synthesize(&input);
        assert_eq!(stmt.recommendation.label(), "PROCEED");
        assert!(stmt.confidence >= 0.50);
    }

    #[test]
    fn conditions_when_high_threat() {
        let input = WisdomInput::new("deploy auth service", "fintech")
            .with_base_confidence(0.75)
            .with_redteam(RedTeamEvidence {
                threat_name: "Credential Exploitation".into(),
                severity: "HIGH".into(),
                risk_score: 0.75,
                mitigation: "Rotate credentials before deployment.".into(),
            });
        let stmt = WisdomStatement::synthesize(&input);
        assert_eq!(stmt.recommendation.label(), "PROCEED-WITH-CONDITIONS");
    }

    #[test]
    fn do_not_proceed_on_critical_threat() {
        let input = WisdomInput::new("deploy with known critical vulnerability", "security")
            .with_base_confidence(0.80)
            .with_redteam(RedTeamEvidence {
                threat_name: "Critical Breach".into(),
                severity: "CRITICAL".into(),
                risk_score: 0.95,
                mitigation: "Do not deploy until patched.".into(),
            });
        let stmt = WisdomStatement::synthesize(&input);
        assert_eq!(stmt.recommendation.label(), "DO-NOT-PROCEED");
    }

    #[test]
    fn calibration_applied_to_confidence() {
        let input = WisdomInput::new("fintech risk assessment", "fintech")
            .with_base_confidence(0.85)
            .with_calibration(CalibrationEvidence {
                raw_confidence: 0.85,
                calibrated_confidence: 0.68,
                bias_direction: "overconfident".into(),
                is_reliable: true,
            });
        let stmt = WisdomStatement::synthesize(&input);
        assert!(stmt.confidence <= 0.85);
    }

    #[test]
    fn reasoning_chain_non_empty_with_signals() {
        let input = WisdomInput::new("deploy", "engineering")
            .with_pattern(PatternEvidence {
                pattern_name: "Cascade Failure".into(),
                is_warning: true,
                similarity: 0.80,
                response: "install circuit breakers".into(),
            })
            .with_base_confidence(0.75);
        let stmt = WisdomStatement::synthesize(&input);
        assert!(!stmt.reasoning_chain.is_empty());
        assert!(!stmt.reversal_conditions.is_empty());
    }
}
