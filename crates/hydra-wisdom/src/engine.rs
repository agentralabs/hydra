//! WisdomEngine — the synthesis coordinator. Layer 4 closes here.

use crate::{
    errors::WisdomError, input::WisdomInput, memory::WisdomMemory, statement::WisdomStatement,
};

/// The wisdom engine.
pub struct WisdomEngine {
    pub memory: WisdomMemory,
    statement_count: usize,
}

impl WisdomEngine {
    pub fn new() -> Self {
        Self {
            memory: WisdomMemory::new(),
            statement_count: 0,
        }
    }

    /// Create a wisdom engine backed by SQLite persistence.
    pub fn open() -> Self {
        Self {
            memory: WisdomMemory::open(),
            statement_count: 0,
        }
    }

    /// Synthesize wisdom from a complete Layer 4 input set.
    pub fn synthesize(&mut self, input: &WisdomInput) -> Result<WisdomStatement, WisdomError> {
        if !input.has_intelligence() {
            return Err(WisdomError::InsufficientIntelligence);
        }

        // Check if similar judgment was made before
        let prior = self.memory.recall_similar(&input.context);
        let prior_context: Option<String> = prior.first().map(|p| {
            if let Some(outcome) = &p.outcome {
                format!(
                    "Prior judgment for similar context: '{}'. \
                     Outcome: {} (was {}). ",
                    p.recommendation,
                    outcome.actual_outcome,
                    if outcome.was_correct {
                        "correct"
                    } else {
                        "incorrect"
                    },
                )
            } else {
                format!(
                    "Prior judgment for similar context: '{}' \
                     (outcome pending). ",
                    p.recommendation,
                )
            }
        });

        // Synthesize the statement
        let mut stmt = WisdomStatement::synthesize(input);

        // Inject prior context into reasoning chain if available
        if let Some(prior_ctx) = prior_context {
            stmt.reasoning_chain.insert(0, prior_ctx);
        }

        // Store in memory
        self.memory.store(&stmt);
        self.statement_count += 1;

        Ok(stmt)
    }

    /// Record outcome for the most recent judgment.
    pub fn record_last_outcome(&mut self, was_correct: bool, actual: impl Into<String>) {
        if let Some(last) = self.memory.entries.last().map(|e| e.id.clone()) {
            self.memory.record_outcome(&last, was_correct, actual);
        }
    }

    pub fn statement_count(&self) -> usize {
        self.statement_count
    }
    pub fn memory_size(&self) -> usize {
        self.memory.count()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "wisdom: judgments={} memories={}",
            self.statement_count,
            self.memory_size(),
        )
    }
}

impl Default for WisdomEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::*;

    fn full_input() -> WisdomInput {
        WisdomInput::new(
            "deploy auth service to production with cert rotation",
            "fintech",
        )
        .with_base_confidence(0.75)
        .with_pattern(PatternEvidence {
            pattern_name: "Cascade Failure".into(),
            is_warning: true,
            similarity: 0.78,
            response: "Install circuit breakers at dependency boundaries.".into(),
        })
        .with_oracle(OracleEvidence {
            scenario_name: "partial failure requiring rollback".into(),
            probability: 0.22,
            is_adverse: true,
            intervention: Some("blue-green deployment".into()),
        })
        .with_redteam(RedTeamEvidence {
            threat_name: "Credential Exploitation".into(),
            severity: "HIGH".into(),
            risk_score: 0.76,
            mitigation: "Rotate credentials before deployment.".into(),
        })
        .with_calibration(CalibrationEvidence {
            raw_confidence: 0.75,
            calibrated_confidence: 0.63,
            bias_direction: "overconfident".into(),
            is_reliable: true,
        })
    }

    #[test]
    fn full_synthesis_produces_statement() {
        let mut engine = WisdomEngine::new();
        let stmt = engine.synthesize(&full_input()).expect("should synthesize");
        assert!(!stmt.recommendation.label().is_empty());
        assert!(!stmt.reasoning_chain.is_empty());
        assert!(stmt.confidence >= 0.0 && stmt.confidence <= 1.0);
        assert_eq!(engine.statement_count(), 1);
    }

    #[test]
    fn critical_threat_blocks_proceed() {
        let mut engine = WisdomEngine::new();
        let input = WisdomInput::new("deploy with critical vulnerability", "security")
            .with_base_confidence(0.85)
            .with_redteam(RedTeamEvidence {
                threat_name: "Zero Day".into(),
                severity: "CRITICAL".into(),
                risk_score: 0.97,
                mitigation: "Patch before deployment.".into(),
            });
        let stmt = engine.synthesize(&input).expect("should synthesize");
        assert_eq!(stmt.recommendation.label(), "DO-NOT-PROCEED");
    }

    #[test]
    fn memory_recalled_on_similar_context() {
        let mut engine = WisdomEngine::new();
        let input1 = WisdomInput::new("deploy auth service production cert rotation", "fintech")
            .with_pattern(PatternEvidence {
                pattern_name: "Trust Escalation".into(),
                is_warning: true,
                similarity: 0.70,
                response: "audit permissions".into(),
            });
        let _stmt1 = engine.synthesize(&input1).expect("should synthesize");
        engine.record_last_outcome(true, "deployment succeeded");

        // Similar context — should recall prior judgment
        let input2 = WisdomInput::new("deploy auth service cert rotation production", "fintech")
            .with_pattern(PatternEvidence {
                pattern_name: "Trust Escalation".into(),
                is_warning: true,
                similarity: 0.70,
                response: "audit permissions".into(),
            });
        let stmt2 = engine.synthesize(&input2).expect("should synthesize");

        let has_prior = stmt2
            .reasoning_chain
            .iter()
            .any(|r| r.contains("Prior") || r.contains("prior"));
        assert!(has_prior, "Reasoning chain should reference prior judgment");
        assert_eq!(engine.statement_count(), 2);
        assert_eq!(engine.memory_size(), 2);
    }

    #[test]
    fn empty_input_returns_error() {
        let mut engine = WisdomEngine::new();
        let input = WisdomInput::new("test", "test");
        let result = engine.synthesize(&input);
        assert!(matches!(result, Err(WisdomError::InsufficientIntelligence)));
    }

    #[test]
    fn summary_format() {
        let engine = WisdomEngine::new();
        let s = engine.summary();
        assert!(s.contains("wisdom:"));
        assert!(s.contains("judgments="));
        assert!(s.contains("memories="));
    }
}
