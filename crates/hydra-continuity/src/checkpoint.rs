//! ContinuityCheckpoint — a single timestamped proof of entity state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A single continuity checkpoint recording the entity's state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityCheckpoint {
    pub id: String,
    pub lineage_id: String,
    pub day: u32,
    pub morphic_hash: String,
    pub soul_count: usize,
    pub genome_count: usize,
    pub notable_change: Option<String>,
    pub checkpoint_hash: String,
    pub recorded_at: DateTime<Utc>,
}

impl ContinuityCheckpoint {
    /// Create a new checkpoint. The checkpoint_hash is computed from all fields.
    pub fn new(
        lineage_id: impl Into<String>,
        day: u32,
        morphic_hash: impl Into<String>,
        soul_count: usize,
        genome_count: usize,
        notable_change: Option<String>,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let lineage = lineage_id.into();
        let morphic = morphic_hash.into();
        let now = Utc::now();

        let checkpoint_hash =
            Self::compute_hash(&id, &lineage, day, &morphic, soul_count, genome_count, &now);

        Self {
            id,
            lineage_id: lineage,
            day,
            morphic_hash: morphic,
            soul_count,
            genome_count,
            notable_change,
            checkpoint_hash,
            recorded_at: now,
        }
    }

    fn compute_hash(
        id: &str,
        lineage_id: &str,
        day: u32,
        morphic_hash: &str,
        soul_count: usize,
        genome_count: usize,
        at: &DateTime<Utc>,
    ) -> String {
        let mut h = Sha256::new();
        h.update(id.as_bytes());
        h.update(lineage_id.as_bytes());
        h.update(day.to_le_bytes());
        h.update(morphic_hash.as_bytes());
        h.update(soul_count.to_le_bytes());
        h.update(genome_count.to_le_bytes());
        h.update(at.to_rfc3339().as_bytes());
        hex::encode(h.finalize())
    }

    /// Verify the checkpoint's integrity hash.
    pub fn verify(&self) -> bool {
        if self.checkpoint_hash.is_empty() || self.checkpoint_hash.len() != 64 {
            return false;
        }
        let expected = Self::compute_hash(
            &self.id,
            &self.lineage_id,
            self.day,
            &self.morphic_hash,
            self.soul_count,
            self.genome_count,
            &self.recorded_at,
        );
        self.checkpoint_hash == expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_hash_valid() {
        let cp = ContinuityCheckpoint::new(
            "hydra-agentra-lineage",
            365,
            "abc123def456",
            100,
            500,
            Some("first annual checkpoint".into()),
        );
        assert!(cp.verify());
        assert_eq!(cp.checkpoint_hash.len(), 64);
        assert_eq!(cp.day, 365);
        assert_eq!(cp.soul_count, 100);
        assert_eq!(cp.genome_count, 500);
    }
}
