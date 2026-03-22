//! Mode tracker — per-domain, per-mode accuracy tracking.

use crate::constants::{
    MODE_RELIABLE_THRESHOLD, MODE_FLAGGED_THRESHOLD, MAX_HISTORY_PER_DOMAIN, MAX_WEIGHT_DELTA,
    MIN_OBSERVATIONS_FOR_ADJUSTMENT, OBSERVATION_WINDOW_SIZE,
};
use crate::observation::{ObservationOutcome, ReasoningObservation};
use crate::record::LearningRecord;
use hydra_reasoning::conclusion::ReasoningMode;
use std::collections::HashMap;

/// Tracks per-mode accuracy within a single domain.
#[derive(Debug, Clone)]
struct ModeAccuracy {
    /// Recent outcomes (true = correct, false = incorrect). Unknown excluded.
    outcomes: Vec<bool>,
}

impl ModeAccuracy {
    /// Create a new empty tracker.
    fn new() -> Self {
        Self {
            outcomes: Vec::new(),
        }
    }

    /// Record an outcome. Unknown outcomes are ignored.
    fn record(&mut self, outcome: &ObservationOutcome) {
        match outcome {
            ObservationOutcome::Correct => self.outcomes.push(true),
            ObservationOutcome::Incorrect => self.outcomes.push(false),
            ObservationOutcome::Unknown => {}
        }
        // Keep only the most recent observations within the window.
        if self.outcomes.len() > OBSERVATION_WINDOW_SIZE {
            let excess = self.outcomes.len() - OBSERVATION_WINDOW_SIZE;
            self.outcomes.drain(..excess);
        }
    }

    /// Compute running accuracy over recorded outcomes.
    fn accuracy(&self) -> Option<f64> {
        if self.outcomes.is_empty() {
            return None;
        }
        let correct = self.outcomes.iter().filter(|&&o| o).count();
        Some(correct as f64 / self.outcomes.len() as f64)
    }

    /// Return the total number of scored outcomes.
    fn count(&self) -> usize {
        self.outcomes.len()
    }
}

/// Per-domain accuracy tracking across all reasoning modes.
#[derive(Debug, Clone)]
struct DomainTracker {
    /// Per-mode accuracy trackers.
    modes: HashMap<String, ModeAccuracy>,
    /// Total observations recorded for this domain.
    total_observations: usize,
}

impl DomainTracker {
    /// Create a new domain tracker.
    fn new() -> Self {
        Self {
            modes: HashMap::new(),
            total_observations: 0,
        }
    }

    /// Record an observation for this domain.
    fn record(&mut self, observation: &ReasoningObservation) {
        if self.total_observations >= MAX_HISTORY_PER_DOMAIN {
            return;
        }
        for mode in &observation.contributing_modes {
            let entry = self
                .modes
                .entry(mode.label().to_string())
                .or_insert_with(ModeAccuracy::new);
            // Only the primary mode gets the outcome; others get Unknown.
            if observation.primary_mode.as_ref() == Some(mode) {
                entry.record(&observation.outcome);
            }
        }
        self.total_observations += 1;
    }

    /// Get accuracy for a specific mode in this domain.
    fn accuracy_for(&self, mode_label: &str) -> Option<f64> {
        self.modes.get(mode_label).and_then(|m| m.accuracy())
    }
}

/// Tracks reasoning mode accuracy across all domains and produces learning records.
#[derive(Debug)]
pub struct ModeTracker {
    /// Per-domain trackers.
    domains: HashMap<String, DomainTracker>,
}

impl ModeTracker {
    /// Create a new empty mode tracker.
    pub fn new() -> Self {
        Self {
            domains: HashMap::new(),
        }
    }

    /// Record a reasoning observation.
    pub fn record(&mut self, observation: &ReasoningObservation) {
        let tracker = self
            .domains
            .entry(observation.domain.clone())
            .or_insert_with(DomainTracker::new);
        tracker.record(observation);
    }

    /// Get accuracy for a specific mode in a specific domain.
    pub fn accuracy_for(&self, domain: &str, mode: &str) -> Option<f64> {
        self.domains.get(domain).and_then(|d| d.accuracy_for(mode))
    }

    /// Return total observations across all domains.
    pub fn total_observations(&self) -> usize {
        self.domains.values().map(|d| d.total_observations).sum()
    }

    /// Produce learning records for a specific domain.
    ///
    /// Returns an empty vec if there are insufficient observations.
    pub fn check_adjustments(&self, domain: &str) -> Vec<LearningRecord> {
        let tracker = match self.domains.get(domain) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut records = Vec::new();
        for (mode_label, mode_acc) in &tracker.modes {
            if mode_acc.count() < MIN_OBSERVATIONS_FOR_ADJUSTMENT {
                continue;
            }
            let accuracy = match mode_acc.accuracy() {
                Some(a) => a,
                None => continue,
            };
            let mode = match mode_label.as_str() {
                "deductive" => ReasoningMode::Deductive,
                "inductive" => ReasoningMode::Inductive,
                "abductive" => ReasoningMode::Abductive,
                "analogical" => ReasoningMode::Analogical,
                "adversarial" => ReasoningMode::Adversarial,
                _ => continue,
            };

            let (delta, reason) = if accuracy >= MODE_RELIABLE_THRESHOLD {
                let delta = MAX_WEIGHT_DELTA * (accuracy - MODE_RELIABLE_THRESHOLD)
                    / (1.0 - MODE_RELIABLE_THRESHOLD);
                (
                    delta,
                    format!("high accuracy ({accuracy:.2}) warrants boost"),
                )
            } else if accuracy <= MODE_FLAGGED_THRESHOLD {
                let delta = -MAX_WEIGHT_DELTA * (MODE_FLAGGED_THRESHOLD - accuracy)
                    / MODE_FLAGGED_THRESHOLD;
                (
                    delta,
                    format!("low accuracy ({accuracy:.2}) warrants reduction"),
                )
            } else {
                continue;
            };

            // Current weight is 1.0 as default; the actual weight comes from
            // the reasoning engine config, but we observe from the outside.
            let current_weight = 1.0;
            let confidence =
                (mode_acc.count() as f64 / MIN_OBSERVATIONS_FOR_ADJUSTMENT as f64).min(1.0);

            records.push(LearningRecord::new(
                mode,
                domain,
                current_weight,
                delta,
                reason,
                confidence,
            ));
        }
        records
    }

    /// Return the number of tracked domains.
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }
}

impl Default for ModeTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::ObservationOutcome;

    fn make_obs(
        domain: &str,
        mode: ReasoningMode,
        outcome: ObservationOutcome,
    ) -> ReasoningObservation {
        ReasoningObservation {
            id: uuid::Uuid::new_v4().to_string(),
            contributing_modes: vec![mode.clone()],
            primary_mode: Some(mode),
            domain: domain.to_string(),
            intent_type: "test".to_string(),
            outcome,
            timestamp: chrono::Utc::now(),
            synthesis_confidence: 0.7,
        }
    }

    #[test]
    fn records_and_tracks() {
        let mut tracker = ModeTracker::new();
        let obs = make_obs("eng", ReasoningMode::Deductive, ObservationOutcome::Correct);
        tracker.record(&obs);
        assert_eq!(tracker.total_observations(), 1);
    }

    #[test]
    fn accuracy_computed() {
        let mut tracker = ModeTracker::new();
        for _ in 0..3 {
            tracker.record(&make_obs(
                "eng",
                ReasoningMode::Deductive,
                ObservationOutcome::Correct,
            ));
        }
        tracker.record(&make_obs(
            "eng",
            ReasoningMode::Deductive,
            ObservationOutcome::Incorrect,
        ));
        let acc = tracker.accuracy_for("eng", "deductive");
        assert!(acc.is_some());
        assert!((acc.unwrap_or(0.0) - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn insufficient_observations_no_records() {
        let mut tracker = ModeTracker::new();
        for _ in 0..3 {
            tracker.record(&make_obs(
                "eng",
                ReasoningMode::Deductive,
                ObservationOutcome::Correct,
            ));
        }
        let records = tracker.check_adjustments("eng");
        assert!(records.is_empty());
    }

    #[test]
    fn boost_proposed_for_high_accuracy() {
        let mut tracker = ModeTracker::new();
        for _ in 0..6 {
            tracker.record(&make_obs(
                "eng",
                ReasoningMode::Deductive,
                ObservationOutcome::Correct,
            ));
        }
        let records = tracker.check_adjustments("eng");
        assert!(!records.is_empty());
        assert!(records[0].proposed_delta > 0.0);
    }
}
