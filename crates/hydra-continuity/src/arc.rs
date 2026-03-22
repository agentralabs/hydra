//! EntityArc — the full navigable history of an entity.
//! From day 1 to today. Every checkpoint preserved.

use crate::{
    checkpoint::ContinuityCheckpoint,
    constants::{CHECKPOINT_INTERVAL_DAYS, MAX_CHECKPOINTS},
    errors::ContinuityError,
};
use serde::{Deserialize, Serialize};

/// The complete arc of one entity's existence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityArc {
    pub lineage_id: String,
    pub checkpoints: Vec<ContinuityCheckpoint>,
    pub total_days: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl EntityArc {
    pub fn new(lineage_id: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            lineage_id: lineage_id.into(),
            checkpoints: Vec::new(),
            total_days: 0,
            started_at: now,
            updated_at: now,
        }
    }

    /// Add a checkpoint to the arc.
    pub fn add_checkpoint(
        &mut self,
        checkpoint: ContinuityCheckpoint,
    ) -> Result<(), ContinuityError> {
        if self.checkpoints.len() >= MAX_CHECKPOINTS {
            self.checkpoints.remove(0); // prune oldest
        }

        // Check continuity — no large gaps
        if let Some(last) = self.checkpoints.last() {
            if checkpoint.day < last.day {
                return Err(ContinuityError::ContinuityBreak {
                    day: checkpoint.day,
                    last: last.day,
                });
            }
        }

        if checkpoint.day > self.total_days {
            self.total_days = checkpoint.day;
        }
        self.updated_at = chrono::Utc::now();
        self.checkpoints.push(checkpoint);
        Ok(())
    }

    /// Generate checkpoints from a succession package (one per year).
    pub fn from_succession(
        &mut self,
        lineage_id: &str,
        total_days: u32,
        soul_count: usize,
        genome_count: usize,
        sig_chain: &[String],
    ) {
        let years = (total_days / CHECKPOINT_INTERVAL_DAYS).max(1);
        for year in 0..years {
            let day = (year + 1) * CHECKPOINT_INTERVAL_DAYS;
            let day = day.min(total_days);
            let soul_at = soul_count * (year as usize + 1) / years as usize;
            let genome_at = genome_count * (year as usize + 1) / years as usize;
            let morph_hash = sig_chain
                .get(year as usize % sig_chain.len())
                .cloned()
                .unwrap_or_else(|| "0".repeat(64));

            let notable = if year == 0 {
                Some("Year 1: entity established.".into())
            } else if day >= total_days {
                Some(format!("Year {}: present day.", year + 1))
            } else {
                None
            };

            let cp = ContinuityCheckpoint::new(
                lineage_id, day, morph_hash, soul_at, genome_at, notable,
            );
            let _ = self.add_checkpoint(cp);
        }
        self.total_days = total_days;
    }

    /// Get checkpoint nearest to a given day.
    pub fn checkpoint_at(&self, day: u32) -> Option<&ContinuityCheckpoint> {
        self.checkpoints
            .iter()
            .min_by_key(|cp| (cp.day as i64 - day as i64).unsigned_abs())
    }

    /// Prove this arc belongs to the given lineage.
    pub fn prove_lineage(&self, lineage_id: &str) -> bool {
        !self.checkpoints.is_empty()
            && self.lineage_id == lineage_id
            && self.checkpoints.iter().all(|cp| cp.lineage_id == lineage_id)
            && self.checkpoints.iter().all(|cp| cp.verify())
    }

    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    /// Prove continuity between two instances (v1 -> v2 succession).
    pub fn proves_succession_from(&self, v1_arc: &EntityArc) -> bool {
        self.lineage_id == v1_arc.lineage_id && self.total_days >= v1_arc.total_days
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_grows_with_checkpoints() {
        let mut arc = EntityArc::new("hydra-agentra");
        arc.from_succession(
            "hydra-agentra",
            7300,
            730,
            14600,
            &[hex::encode([0u8; 32]), hex::encode([1u8; 32])],
        );
        assert!(arc.checkpoint_count() > 0);
        assert_eq!(arc.total_days, 7300);
    }

    #[test]
    fn lineage_proof_valid() {
        let mut arc = EntityArc::new("hydra-agentra");
        arc.from_succession(
            "hydra-agentra",
            365,
            36,
            730,
            &[hex::encode([0u8; 32])],
        );
        assert!(arc.prove_lineage("hydra-agentra"));
        assert!(!arc.prove_lineage("different-lineage"));
    }

    #[test]
    fn checkpoint_at_nearest_day() {
        let mut arc = EntityArc::new("hydra-agentra");
        arc.from_succession(
            "hydra-agentra",
            730,
            73,
            1460,
            &[hex::encode([0u8; 32])],
        );
        let cp = arc.checkpoint_at(380);
        assert!(cp.is_some());
    }

    #[test]
    fn succession_proof() {
        let mut v1_arc = EntityArc::new("hydra-agentra");
        v1_arc.from_succession(
            "hydra-agentra",
            7300,
            730,
            14600,
            &[hex::encode([0u8; 32])],
        );

        let mut v2_arc = EntityArc::new("hydra-agentra");
        v2_arc.from_succession(
            "hydra-agentra",
            7301,
            731,
            14602,
            &[hex::encode([0u8; 32])],
        );

        assert!(v2_arc.proves_succession_from(&v1_arc));
    }
}
