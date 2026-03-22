//! Checkpoint types — full snapshots and deltas.

use crate::errors::ResurrectionError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Whether a checkpoint is a full snapshot or a delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckpointKind {
    /// Complete state snapshot.
    Full,
    /// Only the fields that changed since the last checkpoint.
    Delta,
}

/// A snapshot of the kernel state fields relevant to checkpointing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelStateSnapshot {
    /// Current Lyapunov stability value.
    pub lyapunov_value: f64,
    /// Step count at capture time.
    pub step_count: u64,
    /// Manifold coordinates.
    pub manifold_coordinates: Vec<f64>,
    /// Average trust level.
    pub average_trust: f64,
    /// Signal queue utilization.
    pub queue_utilization: f64,
    /// Growth rate.
    pub growth_rate: f64,
}

impl KernelStateSnapshot {
    /// Create a snapshot from raw values.
    pub fn new(
        lyapunov_value: f64,
        step_count: u64,
        manifold_coordinates: Vec<f64>,
        average_trust: f64,
        queue_utilization: f64,
        growth_rate: f64,
    ) -> Self {
        Self {
            lyapunov_value,
            step_count,
            manifold_coordinates,
            average_trust,
            queue_utilization,
            growth_rate,
        }
    }

    /// Create a default initial snapshot.
    pub fn initial() -> Self {
        Self {
            lyapunov_value: 1.0,
            step_count: 0,
            manifold_coordinates: vec![0.0; 8],
            average_trust: 0.0,
            queue_utilization: 0.0,
            growth_rate: 0.0,
        }
    }
}

/// A delta capturing only changed fields since the last checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDelta {
    /// New Lyapunov value, if changed.
    pub lyapunov_value: Option<f64>,
    /// New step count, if changed.
    pub step_count: Option<u64>,
    /// New manifold coordinates, if changed.
    pub manifold_coordinates: Option<Vec<f64>>,
    /// New average trust, if changed.
    pub average_trust: Option<f64>,
    /// New queue utilization, if changed.
    pub queue_utilization: Option<f64>,
    /// New growth rate, if changed.
    pub growth_rate: Option<f64>,
}

impl TaskDelta {
    /// Compute the delta between two snapshots.
    pub fn compute(old: &KernelStateSnapshot, new: &KernelStateSnapshot) -> Self {
        Self {
            lyapunov_value: diff_f64(old.lyapunov_value, new.lyapunov_value),
            step_count: if old.step_count != new.step_count {
                Some(new.step_count)
            } else {
                None
            },
            manifold_coordinates: if old.manifold_coordinates != new.manifold_coordinates {
                Some(new.manifold_coordinates.clone())
            } else {
                None
            },
            average_trust: diff_f64(old.average_trust, new.average_trust),
            queue_utilization: diff_f64(old.queue_utilization, new.queue_utilization),
            growth_rate: diff_f64(old.growth_rate, new.growth_rate),
        }
    }

    /// Apply this delta to a snapshot, producing an updated snapshot.
    pub fn apply(&self, base: &KernelStateSnapshot) -> KernelStateSnapshot {
        KernelStateSnapshot {
            lyapunov_value: self.lyapunov_value.unwrap_or(base.lyapunov_value),
            step_count: self.step_count.unwrap_or(base.step_count),
            manifold_coordinates: self
                .manifold_coordinates
                .clone()
                .unwrap_or_else(|| base.manifold_coordinates.clone()),
            average_trust: self.average_trust.unwrap_or(base.average_trust),
            queue_utilization: self.queue_utilization.unwrap_or(base.queue_utilization),
            growth_rate: self.growth_rate.unwrap_or(base.growth_rate),
        }
    }
}

/// Returns `Some(new)` if old and new differ, `None` otherwise.
fn diff_f64(old: f64, new: f64) -> Option<f64> {
    if (old - new).abs() > f64::EPSILON {
        Some(new)
    } else {
        None
    }
}

/// A checkpoint that can be either full or delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique checkpoint ID (monotonically increasing).
    pub id: u64,
    /// What kind of checkpoint this is.
    pub kind: CheckpointKind,
    /// Full snapshot (present only for Full checkpoints).
    pub snapshot: Option<KernelStateSnapshot>,
    /// Delta (present only for Delta checkpoints).
    pub delta: Option<TaskDelta>,
    /// When this checkpoint was created.
    pub created_at: DateTime<Utc>,
    /// SHA256 hash of the serialized payload.
    pub sha256: String,
}

impl Checkpoint {
    /// Create a full checkpoint from a state snapshot.
    pub fn full(id: u64, snapshot: KernelStateSnapshot) -> Result<Self, ResurrectionError> {
        let payload = serde_json::to_string(&snapshot)
            .map_err(|e| ResurrectionError::Serialization(e.to_string()))?;
        let hash = compute_sha256(&payload);
        Ok(Self {
            id,
            kind: CheckpointKind::Full,
            snapshot: Some(snapshot),
            delta: None,
            created_at: Utc::now(),
            sha256: hash,
        })
    }

    /// Create a delta checkpoint.
    pub fn delta(id: u64, delta: TaskDelta) -> Result<Self, ResurrectionError> {
        let payload = serde_json::to_string(&delta)
            .map_err(|e| ResurrectionError::Serialization(e.to_string()))?;
        let hash = compute_sha256(&payload);
        Ok(Self {
            id,
            kind: CheckpointKind::Delta,
            snapshot: None,
            delta: Some(delta),
            created_at: Utc::now(),
            sha256: hash,
        })
    }

    /// Verify the integrity of this checkpoint by recomputing the hash.
    pub fn verify_integrity(&self) -> Result<(), ResurrectionError> {
        let payload = match self.kind {
            CheckpointKind::Full => {
                let snap = self.snapshot.as_ref().ok_or_else(|| {
                    ResurrectionError::Deserialization(
                        "full checkpoint missing snapshot".to_string(),
                    )
                })?;
                serde_json::to_string(snap)
                    .map_err(|e| ResurrectionError::Serialization(e.to_string()))?
            }
            CheckpointKind::Delta => {
                let d = self.delta.as_ref().ok_or_else(|| {
                    ResurrectionError::Deserialization("delta checkpoint missing delta".to_string())
                })?;
                serde_json::to_string(d)
                    .map_err(|e| ResurrectionError::Serialization(e.to_string()))?
            }
        };
        let actual = compute_sha256(&payload);
        if actual != self.sha256 {
            return Err(ResurrectionError::IntegrityFailure {
                expected: self.sha256.clone(),
                actual,
            });
        }
        Ok(())
    }
}

/// Compute a SHA256 hex digest of the given data.
pub fn compute_sha256(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_checkpoint_verifies() {
        let snap = KernelStateSnapshot::initial();
        let cp = Checkpoint::full(1, snap).expect("create full");
        assert!(cp.verify_integrity().is_ok());
    }

    #[test]
    fn tampered_checkpoint_fails() {
        let snap = KernelStateSnapshot::initial();
        let mut cp = Checkpoint::full(1, snap).expect("create full");
        cp.sha256 = "deadbeef".to_string();
        assert!(cp.verify_integrity().is_err());
    }

    #[test]
    fn delta_computation_and_apply() {
        let snap1 = KernelStateSnapshot::initial();
        let mut snap2 = snap1.clone();
        snap2.lyapunov_value = 0.5;
        snap2.step_count = 10;
        let delta = TaskDelta::compute(&snap1, &snap2);
        assert!(delta.lyapunov_value.is_some());
        assert!(delta.step_count.is_some());
        let applied = delta.apply(&snap1);
        assert!((applied.lyapunov_value - 0.5).abs() < f64::EPSILON);
        assert_eq!(applied.step_count, 10);
    }
}
