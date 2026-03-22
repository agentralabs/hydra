//! Warm restart — reconstruct state and check time target.

use crate::checkpoint::{Checkpoint, KernelStateSnapshot};
use crate::constants::WARM_RESTART_TARGET_SECONDS;
use crate::errors::ResurrectionError;
use crate::reader::{CheckpointReader, ReconstructedState};
use std::time::Instant;

/// The result of a warm restart attempt.
#[derive(Debug)]
pub struct RestartResult {
    /// The reconstructed state snapshot.
    pub state: KernelStateSnapshot,
    /// How long the restart took in milliseconds.
    pub elapsed_ms: u64,
    /// Whether the restart met the time target.
    pub met_target: bool,
    /// How many checkpoints were applied.
    pub checkpoints_applied: usize,
    /// How many corrupted checkpoints were skipped.
    pub corrupted_skipped: usize,
}

/// Perform a warm restart from a sequence of checkpoints.
pub fn warm_restart(checkpoints: &[Checkpoint]) -> Result<RestartResult, ResurrectionError> {
    let start = Instant::now();

    let ReconstructedState {
        state,
        checkpoints_applied,
        corrupted_skipped,
    } = CheckpointReader::reconstruct(checkpoints)?;

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_millis() as u64;
    let target_ms = WARM_RESTART_TARGET_SECONDS * 1_000;
    let met_target = elapsed_ms <= target_ms;

    if !met_target {
        tracing::warn!(elapsed_ms, target_ms, "warm restart exceeded time target");
    }

    Ok(RestartResult {
        state,
        elapsed_ms,
        met_target,
        checkpoints_applied,
        corrupted_skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::{Checkpoint, KernelStateSnapshot};

    #[test]
    fn warm_restart_meets_target() {
        let snap = KernelStateSnapshot::initial();
        let cp = Checkpoint::full(1, snap).expect("full");
        let result = warm_restart(&[cp]).expect("restart");
        assert!(result.met_target);
        assert_eq!(result.checkpoints_applied, 1);
    }

    #[test]
    fn warm_restart_empty_fails() {
        let result = warm_restart(&[]);
        assert!(result.is_err());
    }
}
