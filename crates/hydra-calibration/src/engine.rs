//! CalibrationEngine — the epistemic calibration coordinator.

use crate::{
    adjuster::{AdjustedConfidence, ConfidenceAdjuster},
    beta::{BetaTrackerStore, EpistemicProfile},
    bias::BiasProfiler,
    constants::*,
    errors::CalibrationError,
    record::{CalibrationRecord, JudgmentType},
};

/// The calibration engine — now with HEFP (Hydra Epistemic Field Protocol).
pub struct CalibrationEngine {
    records: Vec<CalibrationRecord>,
    profiler: BiasProfiler,
    beta_store: BetaTrackerStore,
    db: Option<crate::persistence::CalibrationDb>,
}

impl CalibrationEngine {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            profiler: BiasProfiler::new(),
            beta_store: BetaTrackerStore::new(),
            db: None,
        }
    }

    /// Create an engine backed by SQLite persistence.
    /// Loads all existing records from disk on open.
    pub fn open() -> Self {
        let (db, records) = match crate::persistence::CalibrationDb::open() {
            Ok(db) => {
                let records = db.load_all();
                (Some(db), records)
            }
            Err(e) => {
                eprintln!("hydra: calibration db open failed, running in-memory: {}", e);
                (None, Vec::new())
            }
        };
        let mut profiler = BiasProfiler::new();
        profiler.update_from_records(&records);
        let mut beta_store = BetaTrackerStore::new();
        for r in &records {
            if let Some(outcome) = &r.outcome {
                let success = (outcome.accuracy - r.stated_confidence).abs() < 0.15;
                beta_store.observe(&r.domain, &r.judgment_type, success);
            }
        }
        Self { records, profiler, beta_store, db }
    }

    /// Record a new prediction before its outcome is known.
    pub fn record_prediction(
        &mut self,
        domain: impl Into<String>,
        judgment_type: JudgmentType,
        stated_confidence: f64,
    ) -> Result<String, CalibrationError> {
        if self.records.len() >= MAX_CALIBRATION_RECORDS {
            return Err(CalibrationError::StoreFull {
                max: MAX_CALIBRATION_RECORDS,
            });
        }
        let record = CalibrationRecord::new(domain, judgment_type, stated_confidence);
        let id = record.id.clone();
        if let Some(ref db) = self.db {
            db.insert(&record);
        }
        self.records.push(record);
        Ok(id)
    }

    /// Record the actual outcome for a prediction.
    pub fn record_outcome(
        &mut self,
        record_id: &str,
        actual_accuracy: f64,
    ) -> Result<(), CalibrationError> {
        // Find index first to avoid double borrow
        let idx = self.records.iter().position(|r| r.id == record_id)
            .ok_or_else(|| CalibrationError::InsufficientRecords {
                domain: record_id.to_string(), count: 0, min: 1,
            })?;
        let stated = self.records[idx].stated_confidence;
        let domain = self.records[idx].domain.clone();
        let jtype = self.records[idx].judgment_type.clone();
        self.records[idx].record_outcome(actual_accuracy)?;
        self.profiler.update_from_records(&self.records);
        let success = (actual_accuracy - stated).abs() < 0.15;
        self.beta_store.observe(&domain, &jtype, success);
        Ok(())
    }

    /// Get calibrated confidence for a new judgment.
    pub fn calibrate(
        &self,
        raw_confidence: f64,
        domain: &str,
        judgment_type: &JudgmentType,
    ) -> AdjustedConfidence {
        let adjuster = ConfidenceAdjuster::new(&self.profiler);
        adjuster.adjust(raw_confidence, domain, judgment_type)
    }

    /// HEFP: Get full epistemic profile for a domain/judgment combination.
    pub fn epistemic_profile(&self, domain: &str, judgment_type: &JudgmentType) -> EpistemicProfile {
        self.beta_store.profile(domain, judgment_type)
    }

    /// Overall calibration health score (0.0-1.0).
    /// 1.0 = perfectly calibrated everywhere.
    /// Lower = significant biases detected.
    pub fn calibration_health(&self) -> f64 {
        let significant = self.profiler.significant_biases();
        if significant.is_empty() {
            return 1.0;
        }

        let avg_bias =
            significant.iter().map(|b| b.mean_offset.abs()).sum::<f64>() / significant.len() as f64;

        // Health decreases with bias magnitude
        (1.0 - avg_bias * 2.0).max(0.0)
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }
    pub fn profile_count(&self) -> usize {
        self.profiler.profile_count()
    }
    pub fn significant_bias_count(&self) -> usize {
        self.profiler.significant_biases().len()
    }

    pub fn resolved_records(&self) -> Vec<&CalibrationRecord> {
        self.records.iter().filter(|r| r.has_outcome()).collect()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "calibration: records={} resolved={} biases={} health={:.2}",
            self.record_count(),
            self.resolved_records().len(),
            self.significant_bias_count(),
            self.calibration_health(),
        )
    }
}

impl Default for CalibrationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_engine_with_data(
        domain: &str,
        stated: f64,
        actual: f64,
        n: usize,
    ) -> (CalibrationEngine, Vec<String>) {
        let mut engine = CalibrationEngine::new();
        let mut ids = Vec::new();
        for _ in 0..n {
            let id = engine
                .record_prediction(domain, JudgmentType::RiskAssessment, stated)
                .unwrap();
            ids.push(id);
        }
        for id in &ids {
            engine.record_outcome(id, actual).unwrap();
        }
        (engine, ids)
    }

    #[test]
    fn records_accumulate() {
        let (engine, _) = build_engine_with_data("test", 0.8, 0.7, 5);
        assert_eq!(engine.record_count(), 5);
        assert_eq!(engine.resolved_records().len(), 5);
    }

    #[test]
    fn significant_bias_detected_at_threshold() {
        let (engine, _) = build_engine_with_data("fintech", 0.85, 0.65, MIN_RECORDS_FOR_BIAS);
        // offset = 0.65 - 0.85 = -0.20 (overconfident, significant)
        assert!(engine.significant_bias_count() > 0);
    }

    #[test]
    fn calibration_adjusts_overconfident() {
        let (engine, _) = build_engine_with_data("fintech", 0.85, 0.65, MIN_RECORDS_FOR_BIAS);
        let adjusted = engine.calibrate(0.85, "fintech", &JudgmentType::RiskAssessment);
        assert!(adjusted.calibrated < adjusted.raw);
    }

    #[test]
    fn health_decreases_with_bias() {
        let perfect = CalibrationEngine::new();
        assert_eq!(perfect.calibration_health(), 1.0);

        let (biased, _) = build_engine_with_data("engineering", 0.90, 0.60, MIN_RECORDS_FOR_BIAS);
        assert!(biased.calibration_health() < 1.0);
    }

    #[test]
    fn summary_format() {
        let engine = CalibrationEngine::new();
        let s = engine.summary();
        assert!(s.contains("calibration:"));
        assert!(s.contains("records="));
        assert!(s.contains("health="));
    }
}
