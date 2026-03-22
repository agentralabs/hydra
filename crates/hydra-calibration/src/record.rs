//! CalibrationRecord — one prediction with its stated confidence and
//! eventual actual outcome. The raw material for bias detection.

use serde::{Deserialize, Serialize};

/// The type of judgment being calibrated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JudgmentType {
    RiskAssessment,
    ComplexityEstimate,
    SuccessProbability,
    TimeEstimate,
    TrustAssessment,
    SecurityAssessment,
    Other(String),
}

impl JudgmentType {
    pub fn label(&self) -> String {
        match self {
            Self::RiskAssessment => "risk".into(),
            Self::ComplexityEstimate => "complexity".into(),
            Self::SuccessProbability => "success-prob".into(),
            Self::TimeEstimate => "time".into(),
            Self::TrustAssessment => "trust".into(),
            Self::SecurityAssessment => "security".into(),
            Self::Other(s) => s.clone(),
        }
    }
}

/// The outcome of a prediction — was the stated confidence accurate?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionOutcome {
    /// Was the prediction correct? (true/false for discrete, or accuracy 0-1)
    pub accuracy: f64,
    /// The difference: actual accuracy - stated confidence.
    /// Positive = underconfident (you were less confident than you should be).
    /// Negative = overconfident (you were more confident than warranted).
    pub offset: f64,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

/// One calibration record — prediction + eventual outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationRecord {
    pub id: String,
    pub domain: String,
    pub judgment_type: JudgmentType,
    pub stated_confidence: f64,
    pub outcome: Option<PredictionOutcome>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CalibrationRecord {
    pub fn new(
        domain: impl Into<String>,
        judgment_type: JudgmentType,
        stated_confidence: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            domain: domain.into(),
            judgment_type,
            stated_confidence: stated_confidence.clamp(0.0, 1.0),
            outcome: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Record the actual outcome once it is known.
    pub fn record_outcome(
        &mut self,
        actual_accuracy: f64,
    ) -> Result<(), crate::errors::CalibrationError> {
        if self.outcome.is_some() {
            return Err(crate::errors::CalibrationError::OutcomeAlreadyRecorded {
                id: self.id.clone(),
            });
        }
        let offset = actual_accuracy - self.stated_confidence;
        self.outcome = Some(PredictionOutcome {
            accuracy: actual_accuracy.clamp(0.0, 1.0),
            offset,
            recorded_at: chrono::Utc::now(),
        });
        Ok(())
    }

    pub fn has_outcome(&self) -> bool {
        self.outcome.is_some()
    }
    pub fn offset(&self) -> Option<f64> {
        self.outcome.as_ref().map(|o| o.offset)
    }
    pub fn is_overconfident(&self) -> bool {
        self.offset().map(|o| o < 0.0).unwrap_or(false)
    }
    pub fn is_underconfident(&self) -> bool {
        self.offset().map(|o| o > 0.0).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_outcome_computes_offset() {
        let mut r = CalibrationRecord::new("engineering", JudgmentType::RiskAssessment, 0.85);
        r.record_outcome(0.70).unwrap();
        let offset = r.offset().unwrap();
        // actual (0.70) - stated (0.85) = -0.15 (overconfident)
        assert!((offset - (-0.15)).abs() < 1e-10);
        assert!(r.is_overconfident());
        assert!(!r.is_underconfident());
    }

    #[test]
    fn duplicate_outcome_returns_error() {
        let mut r = CalibrationRecord::new("test", JudgmentType::RiskAssessment, 0.7);
        r.record_outcome(0.6).unwrap();
        let r2 = r.record_outcome(0.8);
        assert!(matches!(
            r2,
            Err(crate::errors::CalibrationError::OutcomeAlreadyRecorded { .. })
        ));
    }

    #[test]
    fn underconfident_when_actual_higher() {
        let mut r = CalibrationRecord::new("test", JudgmentType::SuccessProbability, 0.5);
        r.record_outcome(0.8).unwrap();
        assert!(r.is_underconfident());
    }
}
