//! BiasProfile — systematic offset per domain+judgment_type combination.
//! Built from calibration records. Feeds the confidence adjuster.

use crate::{
    constants::*,
    record::{CalibrationRecord, JudgmentType},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The key for one bias profile entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BiasKey {
    pub domain: String,
    pub judgment_type: String,
}

impl BiasKey {
    pub fn new(domain: &str, judgment_type: &JudgmentType) -> Self {
        Self {
            domain: domain.to_string(),
            judgment_type: judgment_type.label(),
        }
    }
}

/// One bias profile entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasEntry {
    pub key: BiasKey,
    /// Mean offset across all records for this key.
    /// Positive = systematic underconfidence.
    /// Negative = systematic overconfidence.
    pub mean_offset: f64,
    pub sample_size: usize,
    pub std_dev: f64,
    pub is_significant: bool,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl BiasEntry {
    pub fn direction(&self) -> &'static str {
        if self.mean_offset > SIGNIFICANT_BIAS_THRESHOLD {
            "underconfident"
        } else if self.mean_offset < -SIGNIFICANT_BIAS_THRESHOLD {
            "overconfident"
        } else {
            "well-calibrated"
        }
    }

    pub fn is_reliable(&self) -> bool {
        self.sample_size >= MIN_RECORDS_FOR_BIAS
    }
}

/// Builds and maintains bias profiles from calibration records.
pub struct BiasProfiler {
    profiles: HashMap<BiasKey, BiasEntry>,
}

impl BiasProfiler {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    /// Update profiles from a set of resolved calibration records.
    pub fn update_from_records(&mut self, records: &[CalibrationRecord]) {
        // Group records by bias key
        let mut groups: HashMap<BiasKey, Vec<f64>> = HashMap::new();

        for record in records {
            if let Some(outcome) = &record.outcome {
                let key = BiasKey::new(&record.domain, &record.judgment_type);
                groups.entry(key).or_default().push(outcome.offset);
            }
        }

        // Compute statistics for each group
        for (key, offsets) in &groups {
            if offsets.is_empty() {
                continue;
            }

            let n = offsets.len() as f64;
            let mean = offsets.iter().sum::<f64>() / n;
            let variance = offsets.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
            let std_dev = variance.sqrt();
            let is_sig =
                mean.abs() >= SIGNIFICANT_BIAS_THRESHOLD && offsets.len() >= MIN_RECORDS_FOR_BIAS;

            self.profiles.insert(
                key.clone(),
                BiasEntry {
                    key: key.clone(),
                    mean_offset: mean,
                    sample_size: offsets.len(),
                    std_dev,
                    is_significant: is_sig,
                    updated_at: chrono::Utc::now(),
                },
            );
        }
    }

    pub fn get(&self, key: &BiasKey) -> Option<&BiasEntry> {
        self.profiles.get(key)
    }

    pub fn significant_biases(&self) -> Vec<&BiasEntry> {
        self.profiles
            .values()
            .filter(|e| e.is_significant)
            .collect()
    }

    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }
}

impl Default for BiasProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_records_with_bias(
        domain: &str,
        jtype: JudgmentType,
        stated: f64,
        actual: f64,
        n: usize,
    ) -> Vec<CalibrationRecord> {
        (0..n)
            .map(|_| {
                let mut r = CalibrationRecord::new(domain, jtype.clone(), stated);
                r.record_outcome(actual).unwrap();
                r
            })
            .collect()
    }

    #[test]
    fn overconfidence_detected() {
        let records = make_records_with_bias(
            "engineering",
            JudgmentType::RiskAssessment,
            0.85,
            0.65,
            MIN_RECORDS_FOR_BIAS,
        );
        let mut profiler = BiasProfiler::new();
        profiler.update_from_records(&records);

        let key = BiasKey::new("engineering", &JudgmentType::RiskAssessment);
        let entry = profiler.get(&key).unwrap();
        // mean_offset = 0.65 - 0.85 = -0.20 (overconfident)
        assert!(entry.mean_offset < -SIGNIFICANT_BIAS_THRESHOLD);
        assert_eq!(entry.direction(), "overconfident");
        assert!(entry.is_significant);
    }

    #[test]
    fn underconfidence_detected() {
        let records = make_records_with_bias(
            "security",
            JudgmentType::SecurityAssessment,
            0.50,
            0.75,
            MIN_RECORDS_FOR_BIAS,
        );
        let mut profiler = BiasProfiler::new();
        profiler.update_from_records(&records);

        let key = BiasKey::new("security", &JudgmentType::SecurityAssessment);
        let entry = profiler.get(&key).unwrap();
        assert!(entry.mean_offset > SIGNIFICANT_BIAS_THRESHOLD);
        assert_eq!(entry.direction(), "underconfident");
    }

    #[test]
    fn below_threshold_not_significant() {
        let records = make_records_with_bias(
            "test",
            JudgmentType::RiskAssessment,
            0.75,
            0.77,
            MIN_RECORDS_FOR_BIAS,
        ); // tiny offset of 0.02
        let mut profiler = BiasProfiler::new();
        profiler.update_from_records(&records);

        let key = BiasKey::new("test", &JudgmentType::RiskAssessment);
        let entry = profiler.get(&key).unwrap();
        assert!(!entry.is_significant);
        assert_eq!(entry.direction(), "well-calibrated");
    }
}
