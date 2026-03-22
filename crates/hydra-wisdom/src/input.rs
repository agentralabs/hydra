//! WisdomInput — all Layer 4 intelligence collected for one decision.
//! This is what feeds the synthesis engine.

use serde::{Deserialize, Serialize};

/// Pattern evidence from hydra-pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEvidence {
    pub pattern_name: String,
    pub is_warning: bool,
    pub similarity: f64,
    pub response: String,
}

/// Oracle scenario from hydra-oracle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleEvidence {
    pub scenario_name: String,
    pub probability: f64,
    pub is_adverse: bool,
    pub intervention: Option<String>,
}

/// Red team finding from hydra-redteam.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedTeamEvidence {
    pub threat_name: String,
    pub severity: String,
    pub risk_score: f64,
    pub mitigation: String,
}

/// Calibration data from hydra-calibration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationEvidence {
    pub raw_confidence: f64,
    pub calibrated_confidence: f64,
    pub bias_direction: String,
    pub is_reliable: bool,
}

/// All intelligence collected for one decision.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WisdomInput {
    pub context: String,
    pub domain: String,
    pub patterns: Vec<PatternEvidence>,
    pub oracle: Vec<OracleEvidence>,
    pub redteam: Vec<RedTeamEvidence>,
    pub calibration: Option<CalibrationEvidence>,
    /// Base confidence from reasoning engine.
    pub base_confidence: f64,
}

impl WisdomInput {
    pub fn new(context: impl Into<String>, domain: impl Into<String>) -> Self {
        Self {
            context: context.into(),
            domain: domain.into(),
            base_confidence: 0.70,
            ..Default::default()
        }
    }

    pub fn with_pattern(mut self, p: PatternEvidence) -> Self {
        self.patterns.push(p);
        self
    }

    pub fn with_oracle(mut self, o: OracleEvidence) -> Self {
        self.oracle.push(o);
        self
    }

    pub fn with_redteam(mut self, r: RedTeamEvidence) -> Self {
        self.redteam.push(r);
        self
    }

    pub fn with_calibration(mut self, c: CalibrationEvidence) -> Self {
        self.calibration = Some(c);
        self
    }

    pub fn with_base_confidence(mut self, c: f64) -> Self {
        self.base_confidence = c.clamp(0.0, 1.0);
        self
    }

    pub fn has_intelligence(&self) -> bool {
        !self.patterns.is_empty()
            || !self.oracle.is_empty()
            || !self.redteam.is_empty()
            || self.calibration.is_some()
    }

    /// Adverse signal count — how many inputs recommend caution.
    pub fn adverse_signal_count(&self) -> usize {
        let pattern_warnings = self.patterns.iter().filter(|p| p.is_warning).count();
        let oracle_adverse = self.oracle.iter().filter(|o| o.is_adverse).count();
        let redteam_high = self.redteam.iter().filter(|r| r.risk_score >= 0.5).count();
        pattern_warnings + oracle_adverse + redteam_high
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adverse_signals_counted() {
        let input = WisdomInput::new("deploy auth service", "fintech")
            .with_pattern(PatternEvidence {
                pattern_name: "Cascade Failure".into(),
                is_warning: true,
                similarity: 0.85,
                response: "circuit breakers".into(),
            })
            .with_oracle(OracleEvidence {
                scenario_name: "partial failure".into(),
                probability: 0.25,
                is_adverse: true,
                intervention: Some("canary deploy".into()),
            })
            .with_redteam(RedTeamEvidence {
                threat_name: "Credential Exploitation".into(),
                severity: "HIGH".into(),
                risk_score: 0.80,
                mitigation: "rotate credentials".into(),
            });
        assert_eq!(input.adverse_signal_count(), 3);
    }

    #[test]
    fn no_intelligence_fails_check() {
        let input = WisdomInput::new("test", "test");
        assert!(!input.has_intelligence());
    }
}
