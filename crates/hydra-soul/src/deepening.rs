//! Constitutional deepening — the process of adding new constitutional principles.
//!
//! Deepening is a multi-step lifecycle with a mandatory reflection period.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{DEEPENING_MIN_REFLECTION_DAYS, MAX_DEEPENING_RECORDS};
use crate::errors::SoulError;

/// The state machine for a deepening proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeepeningState {
    /// Newly proposed, awaiting reflection period.
    Proposed,
    /// Reflection period elapsed, awaiting reconfirmation.
    AwaitingReconfirmation,
    /// Reconfirmed, awaiting coherence assessment.
    AwaitingCoherenceAssessment,
    /// Coherence assessed, awaiting final confirmation.
    AwaitingFinalConfirmation,
    /// Fully deepened — now part of the constitution.
    Deepened,
    /// Rejected at any stage.
    Rejected,
}

/// A single deepening record tracking the lifecycle of a proposed principle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepeningRecord {
    /// Unique identifier.
    pub id: String,
    /// The proposed principle text.
    pub principle: String,
    /// Current lifecycle state.
    pub state: DeepeningState,
    /// When the proposal was created.
    pub proposed_at: DateTime<Utc>,
    /// Minimum reflection days required (parameterizable for tests).
    pub min_reflection_days: i64,
    /// Coherence score (0.0-1.0), set during assessment.
    pub coherence_score: Option<f64>,
}

impl DeepeningRecord {
    /// Create a new deepening proposal.
    pub fn propose(principle: impl Into<String>, min_reflection_days: i64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            principle: principle.into(),
            state: DeepeningState::Proposed,
            proposed_at: Utc::now(),
            min_reflection_days,
            coherence_score: None,
        }
    }

    /// Check whether the reflection period has elapsed.
    pub fn reflection_elapsed(&self) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.proposed_at)
            .num_days();
        elapsed >= self.min_reflection_days
    }

    /// Advance from Proposed to AwaitingReconfirmation (requires reflection).
    pub fn reconfirm(&mut self) -> Result<(), SoulError> {
        if self.state != DeepeningState::Proposed
            && self.state != DeepeningState::AwaitingReconfirmation
        {
            return Err(SoulError::DeepeningNotConfirmed);
        }
        if !self.reflection_elapsed() {
            let elapsed = Utc::now()
                .signed_duration_since(self.proposed_at)
                .num_days();
            return Err(SoulError::ReflectionPeriodNotElapsed {
                need_days: self.min_reflection_days,
                have_days: elapsed,
            });
        }
        self.state = DeepeningState::AwaitingCoherenceAssessment;
        Ok(())
    }

    /// Assess coherence with existing constitution.
    pub fn assess_coherence(&mut self, score: f64) -> Result<(), SoulError> {
        if self.state != DeepeningState::AwaitingCoherenceAssessment {
            return Err(SoulError::DeepeningNotConfirmed);
        }
        self.coherence_score = Some(score);
        self.state = DeepeningState::AwaitingFinalConfirmation;
        Ok(())
    }

    /// Finalize the deepening (accept or reject based on confirmation).
    pub fn finalize(&mut self, confirmed: bool) -> Result<(), SoulError> {
        if self.state != DeepeningState::AwaitingFinalConfirmation {
            return Err(SoulError::DeepeningNotConfirmed);
        }
        self.state = if confirmed {
            DeepeningState::Deepened
        } else {
            DeepeningState::Rejected
        };
        Ok(())
    }
}

/// Storage for deepening records.
#[derive(Debug, Clone, Default)]
pub struct DeepeningStore {
    records: Vec<DeepeningRecord>,
}

impl DeepeningStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Propose a new deepening. Uses the default reflection period.
    pub fn propose(&mut self, principle: impl Into<String>) -> Result<String, SoulError> {
        self.propose_with_reflection(principle, DEEPENING_MIN_REFLECTION_DAYS)
    }

    /// Propose a new deepening with a custom reflection period.
    pub fn propose_with_reflection(
        &mut self,
        principle: impl Into<String>,
        min_reflection_days: i64,
    ) -> Result<String, SoulError> {
        if self.records.len() >= MAX_DEEPENING_RECORDS {
            return Err(SoulError::GraphAtCapacity(self.records.len()));
        }
        let record = DeepeningRecord::propose(principle, min_reflection_days);
        let id = record.id.clone();
        self.records.push(record);
        Ok(id)
    }

    /// Get a deepening record by ID.
    pub fn get(&self, id: &str) -> Option<&DeepeningRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    /// Get a mutable deepening record by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut DeepeningRecord> {
        self.records.iter_mut().find(|r| r.id == id)
    }

    /// Return all deepened records.
    pub fn all_deepened(&self) -> Vec<&DeepeningRecord> {
        self.records
            .iter()
            .filter(|r| r.state == DeepeningState::Deepened)
            .collect()
    }

    /// Return the currently active (non-terminal) proposals.
    pub fn active(&self) -> Vec<&DeepeningRecord> {
        self.records
            .iter()
            .filter(|r| r.state != DeepeningState::Deepened && r.state != DeepeningState::Rejected)
            .collect()
    }

    /// Total number of deepened principles.
    pub fn total_deepened(&self) -> usize {
        self.all_deepened().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_lifecycle_zero_reflection() {
        let mut store = DeepeningStore::new();
        let id = store
            .propose_with_reflection("Test principle", 0)
            .expect("propose");

        let rec = store.get_mut(&id).expect("find");
        assert_eq!(rec.state, DeepeningState::Proposed);

        rec.reconfirm().expect("reconfirm");
        assert_eq!(rec.state, DeepeningState::AwaitingCoherenceAssessment);

        rec.assess_coherence(0.95).expect("assess");
        assert_eq!(rec.state, DeepeningState::AwaitingFinalConfirmation);

        rec.finalize(true).expect("finalize");
        assert_eq!(rec.state, DeepeningState::Deepened);

        assert_eq!(store.total_deepened(), 1);
    }

    #[test]
    fn rejection_works() {
        let mut store = DeepeningStore::new();
        let id = store
            .propose_with_reflection("Bad principle", 0)
            .expect("propose");

        let rec = store.get_mut(&id).expect("find");
        rec.reconfirm().expect("reconfirm");
        rec.assess_coherence(0.3).expect("assess");
        rec.finalize(false).expect("finalize");
        assert_eq!(rec.state, DeepeningState::Rejected);
        assert_eq!(store.total_deepened(), 0);
    }

    #[test]
    fn reflection_period_enforced() {
        let mut store = DeepeningStore::new();
        let id = store
            .propose_with_reflection("Needs time", 365)
            .expect("propose");

        let rec = store.get_mut(&id).expect("find");
        let result = rec.reconfirm();
        assert!(result.is_err(), "should fail — reflection not elapsed");
    }
}
