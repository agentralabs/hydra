//! Checkpoint writer — produces full or delta checkpoints based on index state.

use crate::checkpoint::{Checkpoint, CheckpointKind, KernelStateSnapshot, TaskDelta};
use crate::constants::MAX_CHECKPOINT_SIZE_BYTES;
use crate::errors::ResurrectionError;
use crate::index::CheckpointIndex;

/// Writes checkpoints, deciding between full and delta based on index state.
#[derive(Debug)]
pub struct CheckpointWriter {
    /// The last snapshot used as a delta base.
    last_snapshot: Option<KernelStateSnapshot>,
}

impl CheckpointWriter {
    /// Create a new writer with no prior snapshot.
    pub fn new() -> Self {
        Self {
            last_snapshot: None,
        }
    }

    /// Write a checkpoint for the given state.
    ///
    /// If the index says a full checkpoint is needed (or no prior snapshot exists),
    /// a full checkpoint is produced. Otherwise, a delta is computed.
    ///
    /// The checkpoint is registered in the index BEFORE being returned
    /// (write-ahead: index updated first).
    pub fn write(
        &mut self,
        snapshot: &KernelStateSnapshot,
        index: &mut CheckpointIndex,
    ) -> Result<Checkpoint, ResurrectionError> {
        let needs_full = index.needs_full() || self.last_snapshot.is_none();
        let next_id = index.next_id();

        let checkpoint = if needs_full {
            let cp = Checkpoint::full(next_id, snapshot.clone())?;
            self.check_size(&cp)?;
            // Write-ahead: register in index before returning
            index.register(true, cp.sha256.clone());
            self.last_snapshot = Some(snapshot.clone());
            cp
        } else {
            let base = self
                .last_snapshot
                .as_ref()
                .ok_or(ResurrectionError::NoCheckpoints)?;
            let delta = TaskDelta::compute(base, snapshot);
            let cp = Checkpoint::delta(next_id, delta)?;
            self.check_size(&cp)?;
            // Write-ahead: register in index before returning
            index.register(false, cp.sha256.clone());
            self.last_snapshot = Some(snapshot.clone());
            cp
        };

        Ok(checkpoint)
    }

    /// Return the kind of checkpoint that would be written next.
    pub fn next_kind(&self, index: &CheckpointIndex) -> CheckpointKind {
        if index.needs_full() || self.last_snapshot.is_none() {
            CheckpointKind::Full
        } else {
            CheckpointKind::Delta
        }
    }

    /// Check that the checkpoint doesn't exceed the size limit.
    fn check_size(&self, checkpoint: &Checkpoint) -> Result<(), ResurrectionError> {
        let serialized = serde_json::to_string(checkpoint)
            .map_err(|e| ResurrectionError::Serialization(e.to_string()))?;
        let size = serialized.len();
        if size > MAX_CHECKPOINT_SIZE_BYTES {
            return Err(ResurrectionError::CheckpointTooLarge {
                size,
                max: MAX_CHECKPOINT_SIZE_BYTES,
            });
        }
        Ok(())
    }
}

impl Default for CheckpointWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_kernel::state::HydraState;

    #[test]
    fn first_write_is_full() {
        let mut writer = CheckpointWriter::new();
        let mut index = CheckpointIndex::new();
        let state = HydraState::initial();
        let snap = KernelStateSnapshot::from_state(&state);
        let cp = writer.write(&snap, &mut index).expect("write");
        assert_eq!(cp.kind, CheckpointKind::Full);
    }

    #[test]
    fn second_write_is_delta() {
        let mut writer = CheckpointWriter::new();
        let mut index = CheckpointIndex::new();
        let state = HydraState::initial();
        let snap = KernelStateSnapshot::from_state(&state);
        let _ = writer.write(&snap, &mut index).expect("write full");
        let mut state2 = state;
        state2.step_count = 1;
        let snap2 = KernelStateSnapshot::from_state(&state2);
        let cp = writer.write(&snap2, &mut index).expect("write delta");
        assert_eq!(cp.kind, CheckpointKind::Delta);
    }

    #[test]
    fn write_ahead_index_updated() {
        let mut writer = CheckpointWriter::new();
        let mut index = CheckpointIndex::new();
        let state = HydraState::initial();
        let snap = KernelStateSnapshot::from_state(&state);
        let cp = writer.write(&snap, &mut index).expect("write");
        // Index should have the entry before we do anything with the checkpoint
        assert_eq!(index.len(), 1);
        assert_eq!(index.last().expect("entry").sha256, cp.sha256);
    }
}
