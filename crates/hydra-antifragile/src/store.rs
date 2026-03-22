//! Append-only antifragile store.

use crate::constants::ANTIFRAGILE_MAX_RECORDS;
use crate::errors::AntifragileError;
use crate::obstacle::ObstacleClass;
use crate::record::ResistanceRecord;
use std::collections::BTreeMap;

/// Key type for the resistance store (serializable obstacle class).
fn class_key(class: &ObstacleClass) -> String {
    format!("{:?}", class)
}

/// Append-only store for obstacle resistance records.
///
/// Records are never deleted. Resistance only grows.
#[derive(Debug)]
pub struct AntifragileStore {
    /// Records keyed by obstacle class string.
    records: BTreeMap<String, ResistanceRecord>,
    /// Total encounters ever (monotonically increasing).
    total_encounters: u64,
}

impl AntifragileStore {
    /// Create an empty antifragile store.
    pub fn new() -> Self {
        Self {
            records: BTreeMap::new(),
            total_encounters: 0,
        }
    }

    /// Get or create a resistance record for an obstacle class.
    ///
    /// If the class has not been encountered before and the store is
    /// at capacity, returns an error.
    pub fn get_or_create(
        &mut self,
        class: &ObstacleClass,
    ) -> Result<&mut ResistanceRecord, AntifragileError> {
        let key = class_key(class);
        if !self.records.contains_key(&key) {
            if self.records.len() >= ANTIFRAGILE_MAX_RECORDS {
                return Err(AntifragileError::StoreFull {
                    max: ANTIFRAGILE_MAX_RECORDS,
                });
            }
            self.records
                .insert(key.clone(), ResistanceRecord::new(class.clone()));
        }
        Ok(self.records.get_mut(&key).expect("just inserted"))
    }

    /// Record an encounter with an obstacle.
    ///
    /// Creates the record if it does not exist.
    pub fn record_encounter(
        &mut self,
        class: &ObstacleClass,
        success: bool,
        approach_used: Option<&str>,
    ) -> Result<(), AntifragileError> {
        let record = self.get_or_create(class)?;
        record.record_encounter(success, approach_used);
        self.total_encounters += 1;
        Ok(())
    }

    /// Get the current resistance for an obstacle class.
    ///
    /// Returns 0.0 if the class has never been encountered.
    pub fn resistance(&self, class: &ObstacleClass) -> f64 {
        self.records
            .get(&class_key(class))
            .map_or(0.0, |r| r.resistance)
    }

    /// Number of distinct obstacle classes encountered.
    pub fn class_count(&self) -> usize {
        self.records.len()
    }

    /// Total encounters across all obstacle classes (monotonically increasing).
    pub fn total_encounters(&self) -> u64 {
        self.total_encounters
    }
}

impl Default for AntifragileStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_query_resistance() {
        let mut store = AntifragileStore::new();
        store
            .record_encounter(&ObstacleClass::RateLimit, true, Some("backoff"))
            .unwrap();
        assert!(store.resistance(&ObstacleClass::RateLimit) > 0.0);
    }

    #[test]
    fn unknown_class_returns_zero() {
        let store = AntifragileStore::new();
        assert!((store.resistance(&ObstacleClass::AuthChallenge)).abs() < f64::EPSILON);
    }

    #[test]
    fn total_encounters_monotonic() {
        let mut store = AntifragileStore::new();
        store
            .record_encounter(&ObstacleClass::RateLimit, true, None)
            .unwrap();
        store
            .record_encounter(&ObstacleClass::NetworkBlock, false, None)
            .unwrap();
        assert_eq!(store.total_encounters(), 2);
    }

    #[test]
    fn class_count_tracks_distinct() {
        let mut store = AntifragileStore::new();
        store
            .record_encounter(&ObstacleClass::RateLimit, true, None)
            .unwrap();
        store
            .record_encounter(&ObstacleClass::RateLimit, true, None)
            .unwrap();
        store
            .record_encounter(&ObstacleClass::AuthChallenge, false, None)
            .unwrap();
        assert_eq!(store.class_count(), 2);
    }
}
