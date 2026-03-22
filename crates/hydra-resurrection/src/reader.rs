//! Checkpoint reader — reconstructs state from a full checkpoint plus deltas.

use crate::checkpoint::{Checkpoint, CheckpointKind, KernelStateSnapshot};
use crate::errors::ResurrectionError;

/// The result of reconstructing state from checkpoints.
#[derive(Debug)]
pub struct ReconstructedState {
    /// The reconstructed kernel state snapshot.
    pub state: KernelStateSnapshot,
    /// How many checkpoints were applied.
    pub checkpoints_applied: usize,
    /// How many corrupted deltas were skipped.
    pub corrupted_skipped: usize,
}

/// Reads and reconstructs state from a sequence of checkpoints.
pub struct CheckpointReader;

impl CheckpointReader {
    /// Reconstruct state from an ordered list of checkpoints.
    pub fn reconstruct(
        checkpoints: &[Checkpoint],
    ) -> Result<ReconstructedState, ResurrectionError> {
        if checkpoints.is_empty() {
            return Err(ResurrectionError::NoCheckpoints);
        }

        let first = &checkpoints[0];
        if first.kind != CheckpointKind::Full {
            return Err(ResurrectionError::Deserialization(
                "first checkpoint must be Full".to_string(),
            ));
        }

        first.verify_integrity()?;
        let base_snapshot = first
            .snapshot
            .as_ref()
            .ok_or_else(|| {
                ResurrectionError::Deserialization("full checkpoint missing snapshot".to_string())
            })?
            .clone();

        let mut current = base_snapshot;
        let mut applied = 1;
        let mut corrupted = 0;

        for cp in &checkpoints[1..] {
            if cp.kind != CheckpointKind::Delta {
                if cp.verify_integrity().is_ok() {
                    if let Some(snap) = &cp.snapshot {
                        current = snap.clone();
                        applied += 1;
                        continue;
                    }
                }
                tracing::warn!(
                    id = cp.id,
                    "skipping non-delta checkpoint with bad integrity"
                );
                corrupted += 1;
                continue;
            }

            if let Err(e) = cp.verify_integrity() {
                tracing::warn!(
                    id = cp.id,
                    error = %e,
                    "skipping corrupted delta checkpoint"
                );
                corrupted += 1;
                continue;
            }

            if let Some(delta) = &cp.delta {
                current = delta.apply(&current);
                applied += 1;
            } else {
                tracing::warn!(
                    id = cp.id,
                    "delta checkpoint missing delta payload, skipping"
                );
                corrupted += 1;
            }
        }

        Ok(ReconstructedState {
            state: current,
            checkpoints_applied: applied,
            corrupted_skipped: corrupted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::{Checkpoint, KernelStateSnapshot, TaskDelta};

    #[test]
    fn reconstruct_full_only() {
        let snap = KernelStateSnapshot::initial();
        let cp = Checkpoint::full(1, snap).expect("create");
        let result = CheckpointReader::reconstruct(&[cp]).expect("reconstruct");
        assert_eq!(result.checkpoints_applied, 1);
        assert_eq!(result.corrupted_skipped, 0);
    }

    #[test]
    fn reconstruct_with_deltas() {
        let snap1 = KernelStateSnapshot::initial();
        let full = Checkpoint::full(1, snap1.clone()).expect("full");

        let mut snap2 = snap1.clone();
        snap2.lyapunov_value = 0.8;
        snap2.step_count = 5;
        let delta = TaskDelta::compute(&snap1, &snap2);
        let delta_cp = Checkpoint::delta(2, delta).expect("delta");

        let result = CheckpointReader::reconstruct(&[full, delta_cp]).expect("ok");
        assert_eq!(result.checkpoints_applied, 2);
        assert!((result.state.lyapunov_value - 0.8).abs() < f64::EPSILON);
        assert_eq!(result.state.step_count, 5);
    }

    #[test]
    fn corrupted_delta_skipped() {
        let snap = KernelStateSnapshot::initial();
        let full = Checkpoint::full(1, snap.clone()).expect("full");

        let mut snap2 = snap.clone();
        snap2.step_count = 3;
        let delta = TaskDelta::compute(&snap, &snap2);
        let mut bad_cp = Checkpoint::delta(2, delta).expect("delta");
        bad_cp.sha256 = "corrupted".to_string();

        let result = CheckpointReader::reconstruct(&[full, bad_cp]).expect("ok");
        assert_eq!(result.corrupted_skipped, 1);
        assert_eq!(result.state.step_count, 0);
    }

    #[test]
    fn empty_fails() {
        let result = CheckpointReader::reconstruct(&[]);
        assert!(result.is_err());
    }
}
