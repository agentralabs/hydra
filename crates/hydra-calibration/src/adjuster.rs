//! ConfidenceAdjuster — apply calibration to produce honest confidence.
//! "My raw confidence is 0.83.
//!  I have a known -0.11 overconfidence bias here.
//!  Calibrated confidence: 0.72."

use crate::{
    bias::{BiasKey, BiasProfiler},
    constants::MAX_BIAS_CORRECTION,
    record::JudgmentType,
};
use serde::{Deserialize, Serialize};

/// The result of a confidence adjustment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedConfidence {
    pub raw: f64,
    pub calibrated: f64,
    pub bias_applied: f64,
    pub domain: String,
    pub judgment: String,
    pub is_reliable: bool,
    pub note: String,
}

impl AdjustedConfidence {
    pub fn changed_significantly(&self) -> bool {
        (self.raw - self.calibrated).abs() >= crate::constants::SIGNIFICANT_BIAS_THRESHOLD
    }
}

/// Adjusts raw confidence using known bias profiles.
pub struct ConfidenceAdjuster<'a> {
    profiler: &'a BiasProfiler,
}

impl<'a> ConfidenceAdjuster<'a> {
    pub fn new(profiler: &'a BiasProfiler) -> Self {
        Self { profiler }
    }

    /// Adjust a raw confidence value using known bias.
    pub fn adjust(
        &self,
        raw: f64,
        domain: &str,
        judgment_type: &JudgmentType,
    ) -> AdjustedConfidence {
        let key = BiasKey::new(domain, judgment_type);
        let raw = raw.clamp(0.0, 1.0);

        match self.profiler.get(&key) {
            Some(bias) if bias.is_significant && bias.is_reliable() => {
                // Apply correction: calibrated = raw + mean_offset
                // but cap the correction to MAX_BIAS_CORRECTION
                let correction = bias
                    .mean_offset
                    .clamp(-MAX_BIAS_CORRECTION, MAX_BIAS_CORRECTION);
                let calibrated = (raw + correction).clamp(0.0, 1.0);

                let note = format!(
                    "Bias correction applied: {} ({:+.2}) in {} for {}",
                    bias.direction(),
                    bias.mean_offset,
                    domain,
                    judgment_type.label(),
                );

                AdjustedConfidence {
                    raw,
                    calibrated,
                    bias_applied: correction,
                    domain: domain.to_string(),
                    judgment: judgment_type.label(),
                    is_reliable: true,
                    note,
                }
            }
            _ => {
                // No reliable bias data — return raw, flag as uncalibrated
                AdjustedConfidence {
                    raw,
                    calibrated: raw,
                    bias_applied: 0.0,
                    domain: domain.to_string(),
                    judgment: judgment_type.label(),
                    is_reliable: false,
                    note: format!(
                        "No calibration data for {} / {} — using raw confidence",
                        domain,
                        judgment_type.label()
                    ),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bias::BiasProfiler, constants::MIN_RECORDS_FOR_BIAS, record::CalibrationRecord};

    fn build_profiler_with_overconfidence() -> BiasProfiler {
        let records: Vec<CalibrationRecord> = (0..MIN_RECORDS_FOR_BIAS)
            .map(|_| {
                let mut r = CalibrationRecord::new("fintech", JudgmentType::RiskAssessment, 0.85);
                r.record_outcome(0.68).unwrap(); // -0.17 offset (overconfident)
                r
            })
            .collect();
        let mut profiler = BiasProfiler::new();
        profiler.update_from_records(&records);
        profiler
    }

    #[test]
    fn overconfidence_lowers_calibrated_score() {
        let profiler = build_profiler_with_overconfidence();
        let adjuster = ConfidenceAdjuster::new(&profiler);
        let result = adjuster.adjust(0.85, "fintech", &JudgmentType::RiskAssessment);

        assert!(result.calibrated < result.raw);
        assert!(result.is_reliable);
        assert!(result.changed_significantly());
        println!(
            "raw={:.2} calibrated={:.2} bias={:+.2}",
            result.raw, result.calibrated, result.bias_applied
        );
    }

    #[test]
    fn no_calibration_data_returns_raw() {
        let profiler = BiasProfiler::new();
        let adjuster = ConfidenceAdjuster::new(&profiler);
        let result = adjuster.adjust(0.75, "unknown-domain", &JudgmentType::RiskAssessment);
        assert_eq!(result.calibrated, result.raw);
        assert!(!result.is_reliable);
    }

    #[test]
    fn calibrated_confidence_bounded() {
        let profiler = build_profiler_with_overconfidence();
        let adjuster = ConfidenceAdjuster::new(&profiler);
        // Even extreme raw values stay in [0.0, 1.0]
        let low = adjuster.adjust(0.01, "fintech", &JudgmentType::RiskAssessment);
        let high = adjuster.adjust(0.99, "fintech", &JudgmentType::RiskAssessment);
        assert!(low.calibrated >= 0.0);
        assert!(high.calibrated <= 1.0);
    }
}
