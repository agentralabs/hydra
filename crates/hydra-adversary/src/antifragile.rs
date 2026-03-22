//! Antifragile resistance records — only grow, never shrink.

use crate::constants::*;
use crate::threat::ThreatClass;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A record of antifragile resistance against a specific threat class.
/// Resistance ONLY grows. It is NEVER deleted or reset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntifragileRecord {
    /// Threat class this record tracks.
    pub class: ThreatClass,
    /// Current resistance level [ANTIFRAGILE_FLOOR, MAX_RESISTANCE].
    resistance: f64,
    /// Total encounters with this threat class.
    pub encounter_count: u64,
    /// Total successful defenses.
    pub defense_wins: u64,
    /// Total defense losses (threat got through).
    pub defense_losses: u64,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// Last encounter time.
    pub last_encounter: Option<DateTime<Utc>>,
}

impl AntifragileRecord {
    /// Create a new antifragile record for a threat class.
    pub fn new(class: ThreatClass) -> Self {
        Self {
            class,
            resistance: INITIAL_RESISTANCE,
            encounter_count: 0,
            defense_wins: 0,
            defense_losses: 0,
            created_at: Utc::now(),
            last_encounter: None,
        }
    }

    /// Return the current resistance level.
    pub fn resistance(&self) -> f64 {
        self.resistance
    }

    /// Record a successful defense. Resistance grows.
    pub fn record_win(&mut self) {
        self.encounter_count += 1;
        self.defense_wins += 1;
        self.last_encounter = Some(Utc::now());
        self.resistance = (self.resistance + RESISTANCE_PER_WIN).min(MAX_RESISTANCE);
    }

    /// Record a defense loss. Resistance does NOT decrease (antifragile invariant).
    pub fn record_loss(&mut self) {
        self.encounter_count += 1;
        self.defense_losses += 1;
        self.last_encounter = Some(Utc::now());
        // Resistance NEVER decreases. This is the antifragile invariant.
        // Floor is always enforced.
        self.resistance = self.resistance.max(ANTIFRAGILE_FLOOR);
    }

    /// Return the win rate (0.0 if no encounters).
    pub fn win_rate(&self) -> f64 {
        if self.encounter_count == 0 {
            return 0.0;
        }
        self.defense_wins as f64 / self.encounter_count as f64
    }
}

/// A store of antifragile records, keyed by threat class.
/// Records are NEVER deleted.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AntifragileStore {
    records: HashMap<ThreatClass, AntifragileRecord>,
}

impl AntifragileStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    /// Get or create a record for a threat class.
    pub fn get_or_create(&mut self, class: ThreatClass) -> &mut AntifragileRecord {
        self.records
            .entry(class)
            .or_insert_with(|| AntifragileRecord::new(class))
    }

    /// Record an encounter (win or loss) for a threat class.
    pub fn record_encounter(&mut self, class: ThreatClass, won: bool) {
        let record = self.get_or_create(class);
        if won {
            record.record_win();
        } else {
            record.record_loss();
        }
    }

    /// Get the resistance for a threat class (0.0 if never encountered).
    pub fn resistance_for(&self, class: &ThreatClass) -> f64 {
        self.records
            .get(class)
            .map(|r| r.resistance())
            .unwrap_or(0.0)
    }

    /// Return the number of tracked threat classes.
    pub fn class_count(&self) -> usize {
        self.records.len()
    }

    /// Return all records (read-only).
    pub fn all_records(&self) -> Vec<&AntifragileRecord> {
        self.records.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resistance_only_grows() {
        let mut record = AntifragileRecord::new(ThreatClass::PromptInjection);
        let initial = record.resistance();
        record.record_win();
        assert!(record.resistance() > initial);
        let after_win = record.resistance();
        record.record_loss();
        assert!(record.resistance() >= after_win);
    }

    #[test]
    fn floor_enforced() {
        let record = AntifragileRecord::new(ThreatClass::Unknown);
        assert!(record.resistance() >= ANTIFRAGILE_FLOOR);
    }
}
