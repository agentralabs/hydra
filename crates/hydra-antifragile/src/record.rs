//! Resistance records for obstacle tracking.

use crate::constants::{INITIAL_RESISTANCE, RESISTANCE_FLOOR, RESISTANCE_PER_WIN};
use crate::obstacle::ObstacleClass;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A record tracking Hydra's resistance to an obstacle class.
///
/// Resistance only grows (never manually decremented). It may
/// decay slightly over time but never below `RESISTANCE_FLOOR`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResistanceRecord {
    /// The class of obstacle.
    pub obstacle_class: ObstacleClass,
    /// Current resistance level (clamped to [0.0, 1.0]).
    pub resistance: f64,
    /// Total encounters with this obstacle class.
    pub encounter_count: u64,
    /// Number of successful navigations (wins).
    pub wins: u64,
    /// The last approach used against this obstacle.
    pub last_approach: Option<String>,
    /// When this record was first created.
    pub created_at: DateTime<Utc>,
    /// When this obstacle was last encountered.
    pub last_encountered_at: DateTime<Utc>,
}

impl ResistanceRecord {
    /// Create a new resistance record for an obstacle class.
    pub fn new(obstacle_class: ObstacleClass) -> Self {
        let now = Utc::now();
        Self {
            obstacle_class,
            resistance: INITIAL_RESISTANCE,
            encounter_count: 0,
            wins: 0,
            last_approach: None,
            created_at: now,
            last_encountered_at: now,
        }
    }

    /// Record an encounter with this obstacle.
    ///
    /// If successful, resistance increases by `RESISTANCE_PER_WIN`.
    /// Resistance is always clamped to [0.0, 1.0] and never drops
    /// below `RESISTANCE_FLOOR` after first encounter.
    pub fn record_encounter(&mut self, success: bool, approach_used: Option<&str>) {
        self.encounter_count += 1;
        self.last_encountered_at = Utc::now();

        if let Some(approach) = approach_used {
            self.last_approach = Some(approach.to_string());
        }

        if success {
            self.wins += 1;
            self.resistance = (self.resistance + RESISTANCE_PER_WIN).clamp(0.0, 1.0);
        } else {
            // Even failures maintain at least the floor.
            self.resistance = self.resistance.max(RESISTANCE_FLOOR);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_record_has_initial_resistance() {
        let r = ResistanceRecord::new(ObstacleClass::RateLimit);
        assert!((r.resistance - INITIAL_RESISTANCE).abs() < f64::EPSILON);
        assert_eq!(r.encounter_count, 0);
        assert_eq!(r.wins, 0);
    }

    #[test]
    fn win_increases_resistance() {
        let mut r = ResistanceRecord::new(ObstacleClass::RateLimit);
        r.record_encounter(true, Some("backoff"));
        assert!(r.resistance > INITIAL_RESISTANCE);
        assert_eq!(r.wins, 1);
        assert_eq!(r.encounter_count, 1);
    }

    #[test]
    fn loss_maintains_floor() {
        let mut r = ResistanceRecord::new(ObstacleClass::RateLimit);
        r.record_encounter(false, None);
        assert!(r.resistance >= RESISTANCE_FLOOR);
    }

    #[test]
    fn resistance_clamped_at_one() {
        let mut r = ResistanceRecord::new(ObstacleClass::AuthChallenge);
        for _ in 0..100 {
            r.record_encounter(true, Some("token-refresh"));
        }
        assert!(r.resistance <= 1.0);
    }

    #[test]
    fn approach_tracked() {
        let mut r = ResistanceRecord::new(ObstacleClass::TimeoutPattern);
        r.record_encounter(true, Some("retry-with-backoff"));
        assert_eq!(r.last_approach.as_deref(), Some("retry-with-backoff"));
    }
}
