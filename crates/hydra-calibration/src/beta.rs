//! Beta-Binomial Bayesian calibration tracker (HEFP Layer 1).
//!
//! Tracks (successes, failures) per domain as Beta(alpha, beta) distribution.
//! Posterior mean = calibrated confidence. Variance = meta-confidence.
//! This replaces simple mean-offset with proper Bayesian reasoning.

use std::collections::HashMap;
use crate::bias::BiasKey;
use crate::record::JudgmentType;

/// Uninformative prior: Beta(2,2) centered at 0.5
const PRIOR_ALPHA: f64 = 2.0;
const PRIOR_BETA: f64 = 2.0;
/// Variance of prior Beta(2,2) — used to normalize meta-confidence
const PRIOR_VARIANCE: f64 = 0.04;

/// Bayesian tracker for a single (domain, judgment_type) pair.
#[derive(Debug, Clone)]
pub struct BetaTracker {
    pub alpha: f64,
    pub beta: f64,
    pub observations: usize,
}

impl BetaTracker {
    pub fn new() -> Self {
        Self { alpha: PRIOR_ALPHA, beta: PRIOR_BETA, observations: 0 }
    }

    pub fn observe(&mut self, success: bool) {
        if success { self.alpha += 1.0; } else { self.beta += 1.0; }
        self.observations += 1;
    }

    /// Posterior mean: the calibrated confidence value
    pub fn mean(&self) -> f64 { self.alpha / (self.alpha + self.beta) }

    /// Posterior variance: uncertainty about the calibration itself
    pub fn variance(&self) -> f64 {
        let ab = self.alpha + self.beta;
        (self.alpha * self.beta) / (ab * ab * (ab + 1.0))
    }

    /// 90% credible interval (normal approximation to Beta)
    pub fn credible_interval_90(&self) -> (f64, f64) {
        let m = self.mean();
        let s = self.variance().sqrt();
        ((m - 1.645 * s).max(0.0), (m + 1.645 * s).min(1.0))
    }

    /// Meta-confidence: how sure we are about the calibration (0.0-1.0)
    /// 0.0 = too few observations, 1.0 = very certain about the calibration
    pub fn meta_confidence(&self) -> f64 {
        if self.observations < 5 { return 0.0; }
        (1.0 - (self.variance() / PRIOR_VARIANCE).min(1.0)).max(0.0)
    }

    /// Citable methodology string
    pub fn methodology(&self) -> String {
        let ci = self.credible_interval_90();
        format!(
            "Beta({:.0},{:.0}) posterior, {} obs, mean={:.0}%, CI90=[{:.0}%-{:.0}%], meta={:.0}%",
            self.alpha, self.beta, self.observations,
            self.mean() * 100.0, ci.0 * 100.0, ci.1 * 100.0,
            self.meta_confidence() * 100.0,
        )
    }
}

/// Epistemic classification for a domain
#[derive(Debug, Clone, PartialEq)]
pub enum EpistemicClass {
    WellCalibrated,
    Uncertain,
    Uncalibrated,
    Irreducible,
}

/// Full epistemic profile for a domain query
#[derive(Debug, Clone)]
pub struct EpistemicProfile {
    pub calibrated_confidence: f64,
    pub credible_interval: (f64, f64),
    pub meta_confidence: f64,
    pub observations: usize,
    pub methodology: String,
    pub epistemic_class: EpistemicClass,
}

/// Collection of Beta trackers per (domain, judgment_type)
#[derive(Debug, Clone)]
pub struct BetaTrackerStore {
    trackers: HashMap<String, BetaTracker>,
}

impl BetaTrackerStore {
    pub fn new() -> Self { Self { trackers: HashMap::new() } }

    pub fn observe(&mut self, domain: &str, judgment_type: &JudgmentType, success: bool) {
        let key = format!("{}:{}", domain, judgment_type.label());
        self.trackers.entry(key).or_insert_with(BetaTracker::new).observe(success);
    }

    pub fn profile(&self, domain: &str, judgment_type: &JudgmentType) -> EpistemicProfile {
        let key = format!("{}:{}", domain, judgment_type.label());
        let tracker = self.trackers.get(&key).cloned().unwrap_or_else(BetaTracker::new);
        let mc = tracker.meta_confidence();
        let epistemic_class = if mc > 0.7 {
            EpistemicClass::WellCalibrated
        } else if mc > 0.3 {
            EpistemicClass::Uncertain
        } else if tracker.observations == 0 {
            EpistemicClass::Uncalibrated
        } else {
            EpistemicClass::Uncertain
        };
        EpistemicProfile {
            calibrated_confidence: tracker.mean(),
            credible_interval: tracker.credible_interval_90(),
            meta_confidence: mc,
            observations: tracker.observations,
            methodology: tracker.methodology(),
            epistemic_class,
        }
    }

    pub fn tracker_count(&self) -> usize { self.trackers.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prior_is_uninformative() {
        let t = BetaTracker::new();
        assert!((t.mean() - 0.5).abs() < 0.001);
    }

    #[test]
    fn observations_shift_posterior() {
        let mut t = BetaTracker::new();
        for _ in 0..20 { t.observe(true); }
        assert!(t.mean() > 0.8);
        assert!(t.meta_confidence() > 0.5);
    }

    #[test]
    fn failures_lower_posterior() {
        let mut t = BetaTracker::new();
        for _ in 0..20 { t.observe(false); }
        assert!(t.mean() < 0.2);
    }

    #[test]
    fn credible_interval_narrows_with_data() {
        let mut few = BetaTracker::new();
        for _ in 0..5 { few.observe(true); }
        let mut many = BetaTracker::new();
        for _ in 0..100 { many.observe(true); }
        let (lo_few, hi_few) = few.credible_interval_90();
        let (lo_many, hi_many) = many.credible_interval_90();
        assert!((hi_few - lo_few) > (hi_many - lo_many));
    }

    #[test]
    fn meta_confidence_grows_with_observations() {
        let mut t = BetaTracker::new();
        assert_eq!(t.meta_confidence(), 0.0);
        for _ in 0..50 { t.observe(true); }
        assert!(t.meta_confidence() > 0.8);
    }

    #[test]
    fn methodology_string_is_citable() {
        let mut t = BetaTracker::new();
        for _ in 0..30 { t.observe(true); }
        for _ in 0..5 { t.observe(false); }
        let m = t.methodology();
        assert!(m.contains("Beta("));
        assert!(m.contains("obs"));
        assert!(m.contains("CI90"));
    }

    #[test]
    fn store_profiles_domains_independently() {
        let mut store = BetaTrackerStore::new();
        for _ in 0..20 { store.observe("rust", &JudgmentType::SuccessProbability, true); }
        for _ in 0..20 { store.observe("finance", &JudgmentType::RiskAssessment, false); }
        let rust = store.profile("rust", &JudgmentType::SuccessProbability);
        let fin = store.profile("finance", &JudgmentType::RiskAssessment);
        assert!(rust.calibrated_confidence > 0.7);
        assert!(fin.calibrated_confidence < 0.3);
    }

    #[test]
    fn uncalibrated_domain_returns_prior() {
        let store = BetaTrackerStore::new();
        let p = store.profile("unknown", &JudgmentType::SuccessProbability);
        assert_eq!(p.epistemic_class, EpistemicClass::Uncalibrated);
        assert!((p.calibrated_confidence - 0.5).abs() < 0.01);
    }
}
