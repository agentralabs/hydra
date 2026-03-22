//! BehaviorPattern — a group of repeated similar executions.
//! When a pattern fires >= CRYSTALLIZATION_THRESHOLD times: propose it.

use crate::{constants::*, observation::ExecutionObservation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A detected behavior pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPattern {
    pub id: String,
    pub signature: String,
    pub action_id: String,
    pub domain: String,
    pub observations: Vec<String>, // observation IDs
    pub count: usize,
    pub avg_duration_ms: f64,
    pub success_rate: f64,
    /// Most common params across observations (for skill generation).
    pub common_params: HashMap<String, String>,
    pub first_seen: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

impl BehaviorPattern {
    pub fn new(obs: &ExecutionObservation) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            signature: obs.signature(),
            action_id: obs.action_id.clone(),
            domain: obs.domain.clone(),
            observations: vec![obs.id.clone()],
            count: 1,
            avg_duration_ms: obs.duration_ms as f64,
            success_rate: if obs.succeeded { 1.0 } else { 0.0 },
            common_params: obs.params.clone(),
            first_seen: now,
            last_seen: now,
        }
    }

    /// Add a new observation to this pattern.
    pub fn add_observation(&mut self, obs: &ExecutionObservation) {
        self.observations.push(obs.id.clone());
        self.count += 1;
        // Update running average duration
        self.avg_duration_ms = (self.avg_duration_ms * (self.count - 1) as f64
            + obs.duration_ms as f64)
            / self.count as f64;
        // Update success rate
        let prev_successes = self.success_rate * (self.count - 1) as f64;
        self.success_rate =
            (prev_successes + if obs.succeeded { 1.0 } else { 0.0 }) / self.count as f64;
        self.last_seen = chrono::Utc::now();
    }

    /// True if this pattern meets the crystallization threshold.
    pub fn is_crystallizable(&self) -> bool {
        self.count >= CRYSTALLIZATION_THRESHOLD && self.success_rate >= 0.5
    }

    /// Human-readable description for the proposal.
    pub fn description(&self) -> String {
        format!(
            "'{}' in {} domain — observed {} times \
             (success rate: {:.0}%, avg {:.0}ms)",
            self.action_id,
            self.domain,
            self.count,
            self.success_rate * 100.0,
            self.avg_duration_ms,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_obs(action_id: &str, succeeded: bool) -> ExecutionObservation {
        ExecutionObservation::new(
            action_id,
            "test intent",
            HashMap::new(),
            "engineering",
            500,
            succeeded,
        )
    }

    #[test]
    fn pattern_starts_below_threshold() {
        let obs = make_obs("deploy.run", true);
        let p = BehaviorPattern::new(&obs);
        assert!(!p.is_crystallizable());
        assert_eq!(p.count, 1);
    }

    #[test]
    fn pattern_crystallizable_at_threshold() {
        let obs = make_obs("deploy.run", true);
        let mut p = BehaviorPattern::new(&obs);
        for _ in 1..CRYSTALLIZATION_THRESHOLD {
            p.add_observation(&make_obs("deploy.run", true));
        }
        assert!(p.is_crystallizable());
        assert_eq!(p.count, CRYSTALLIZATION_THRESHOLD);
    }

    #[test]
    fn low_success_rate_not_crystallizable() {
        let obs = make_obs("flaky.action", true);
        let mut p = BehaviorPattern::new(&obs);
        // Add mostly failures
        for _ in 1..CRYSTALLIZATION_THRESHOLD {
            p.add_observation(&make_obs("flaky.action", false));
        }
        // success_rate < 0.5 → not crystallizable
        assert!(!p.is_crystallizable());
    }
}
