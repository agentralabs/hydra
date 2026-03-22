//! IdentityMemory — accumulates who the principal is from session patterns.
//! Built slowly from all other layers over time.
//! Never deleted. Only deepened.

use crate::{
    constants::IDENTITY_MIN_SESSIONS_FOR_CONFIDENCE,
    layers::{MemoryLayer, MemoryRecord},
};
use serde::{Deserialize, Serialize};

/// A behavioral observation about the principal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralObservation {
    /// The dimension being observed (e.g., "preferred_surface").
    pub dimension: String,
    /// The observed value (e.g., "TUI").
    pub value: String,
    /// Confidence in this observation (0.0 to 1.0).
    pub confidence: f64,
    /// How many times this has been observed.
    pub observation_count: u64,
}

/// The accumulated identity model of the principal.
/// Grows richer over time. Never reset.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IdentityProfile {
    /// How many sessions have been observed.
    pub sessions_observed: u64,
    /// Observed behavioral dimensions.
    pub observations: Vec<BehavioralObservation>,
    /// Estimated peak working hours (hour of day, 0-23).
    pub peak_hours: Vec<u8>,
    /// Average session length in minutes.
    pub avg_session_minutes: f64,
    /// Whether this profile has enough data to be useful.
    pub is_confident: bool,
}

impl IdentityProfile {
    /// Create a new, empty identity profile.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new session into the identity model.
    pub fn observe_session(&mut self, session_duration_minutes: f64, hour_of_day: u8) {
        self.sessions_observed += 1;

        // Update average session length (running average)
        let n = self.sessions_observed as f64;
        self.avg_session_minutes =
            (self.avg_session_minutes * (n - 1.0) + session_duration_minutes) / n;

        // Track peak hours
        if !self.peak_hours.contains(&hour_of_day) {
            self.peak_hours.push(hour_of_day);
            self.peak_hours.sort_unstable();
        }

        // Update confidence once enough sessions observed
        self.is_confident = self.sessions_observed >= IDENTITY_MIN_SESSIONS_FOR_CONFIDENCE as u64;
    }

    /// Add a behavioral observation.
    pub fn observe(
        &mut self,
        dimension: impl Into<String>,
        value: impl Into<String>,
        confidence: f64,
    ) {
        let dim = dimension.into();
        let val = value.into();

        if let Some(existing) = self.observations.iter_mut().find(|o| o.dimension == dim) {
            // Update existing observation
            existing.observation_count += 1;
            let n = existing.observation_count as f64;
            // Running average of confidence
            existing.confidence = (existing.confidence * (n - 1.0) + confidence) / n;
            existing.value = val; // most recent value
        } else {
            self.observations.push(BehavioralObservation {
                dimension: dim,
                value: val,
                confidence,
                observation_count: 1,
            });
        }
    }

    /// Convert to a MemoryRecord for storage.
    pub fn to_memory_record(&self, session_id: &str, causal_root: &str) -> MemoryRecord {
        MemoryRecord::new(
            MemoryLayer::Identity,
            serde_json::to_value(self).unwrap_or(serde_json::Value::Null),
            session_id,
            causal_root,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_profile_not_confident() {
        let p = IdentityProfile::new();
        assert!(!p.is_confident);
        assert_eq!(p.sessions_observed, 0);
    }

    #[test]
    fn becomes_confident_after_min_sessions() {
        let mut p = IdentityProfile::new();
        for _i in 0..IDENTITY_MIN_SESSIONS_FOR_CONFIDENCE {
            p.observe_session(45.0, 9);
        }
        assert!(p.is_confident);
    }

    #[test]
    fn observe_tracks_dimensions() {
        let mut p = IdentityProfile::new();
        p.observe("preferred_surface", "TUI", 0.9);
        p.observe("preferred_surface", "TUI", 0.95);
        assert_eq!(p.observations.len(), 1);
        assert_eq!(p.observations[0].observation_count, 2);
    }

    #[test]
    fn peak_hours_tracked() {
        let mut p = IdentityProfile::new();
        p.observe_session(60.0, 9);
        p.observe_session(60.0, 14);
        p.observe_session(60.0, 9); // repeat
        assert!(p.peak_hours.contains(&9));
        assert!(p.peak_hours.contains(&14));
    }
}
