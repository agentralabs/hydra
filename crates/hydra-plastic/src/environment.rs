//! Environment profiles for plasticity adaptation.

use crate::constants::{DEFAULT_CONFIDENCE, ENVIRONMENT_CONFIDENCE_BOOST};
use crate::mode::ExecutionMode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A profile describing an execution environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentProfile {
    /// The name of the environment.
    pub name: String,
    /// The execution mode for this environment.
    pub mode: ExecutionMode,
    /// Confidence in successfully executing in this environment.
    pub confidence: f64,
    /// Number of encounters with this environment.
    pub encounter_count: u64,
    /// When this profile was first created.
    pub created_at: DateTime<Utc>,
    /// When this profile was last encountered.
    pub last_seen_at: DateTime<Utc>,
}

impl EnvironmentProfile {
    /// Create a new environment profile with default confidence.
    pub fn new(name: impl Into<String>, mode: ExecutionMode) -> Self {
        let now = Utc::now();
        Self {
            name: name.into(),
            mode,
            confidence: DEFAULT_CONFIDENCE,
            encounter_count: 0,
            created_at: now,
            last_seen_at: now,
        }
    }

    /// Record an encounter with this environment.
    ///
    /// Boosts confidence on success, maintains on failure.
    /// Confidence is always clamped to [0.0, 1.0].
    pub fn record_encounter(&mut self, success: bool) {
        self.encounter_count += 1;
        self.last_seen_at = Utc::now();
        if success {
            self.confidence = (self.confidence + ENVIRONMENT_CONFIDENCE_BOOST).clamp(0.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_profile_has_default_confidence() {
        let p = EnvironmentProfile::new("test-env", ExecutionMode::NativeBinary);
        assert!((p.confidence - DEFAULT_CONFIDENCE).abs() < f64::EPSILON);
        assert_eq!(p.encounter_count, 0);
    }

    #[test]
    fn success_boosts_confidence() {
        let mut p = EnvironmentProfile::new("test-env", ExecutionMode::NativeBinary);
        let initial = p.confidence;
        p.record_encounter(true);
        assert!(p.confidence > initial);
        assert_eq!(p.encounter_count, 1);
    }

    #[test]
    fn failure_maintains_confidence() {
        let mut p = EnvironmentProfile::new("test-env", ExecutionMode::NativeBinary);
        let initial = p.confidence;
        p.record_encounter(false);
        assert!((p.confidence - initial).abs() < f64::EPSILON);
        assert_eq!(p.encounter_count, 1);
    }

    #[test]
    fn confidence_clamped() {
        let mut p = EnvironmentProfile::new("test-env", ExecutionMode::NativeBinary);
        for _ in 0..100 {
            p.record_encounter(true);
        }
        assert!(p.confidence <= 1.0);
    }
}
