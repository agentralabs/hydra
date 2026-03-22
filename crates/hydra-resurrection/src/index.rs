//! Checkpoint index — tracks all checkpoints and their metadata.

use crate::constants::DELTAS_PER_FULL_CHECKPOINT;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single entry in the checkpoint index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Checkpoint ID.
    pub id: u64,
    /// Whether this is a full or delta checkpoint.
    pub is_full: bool,
    /// SHA256 hash stored at write time.
    pub sha256: String,
    /// When the checkpoint was written.
    pub created_at: DateTime<Utc>,
}

/// The in-memory checkpoint index.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckpointIndex {
    /// All index entries, ordered by ID.
    entries: Vec<IndexEntry>,
    /// The next checkpoint ID to assign.
    next_id: u64,
}

impl CheckpointIndex {
    /// Create a new empty index.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    /// Register a new checkpoint in the index.
    pub fn register(&mut self, is_full: bool, sha256: String) -> u64 {
        let id = self.next_id;
        self.entries.push(IndexEntry {
            id,
            is_full,
            sha256,
            created_at: Utc::now(),
        });
        self.next_id += 1;
        id
    }

    /// Return the next ID that will be assigned.
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Return the ID of the last full checkpoint, if any.
    pub fn last_full_id(&self) -> Option<u64> {
        self.entries.iter().rev().find(|e| e.is_full).map(|e| e.id)
    }

    /// Count the number of delta checkpoints since the last full checkpoint.
    pub fn deltas_since_full(&self) -> u64 {
        let last_full_pos = self.entries.iter().rposition(|e| e.is_full).unwrap_or(0);
        self.entries[last_full_pos..]
            .iter()
            .filter(|e| !e.is_full)
            .count() as u64
    }

    /// Whether the next checkpoint should be a full checkpoint.
    pub fn needs_full(&self) -> bool {
        if self.entries.is_empty() {
            return true;
        }
        self.deltas_since_full() >= DELTAS_PER_FULL_CHECKPOINT
    }

    /// Return all entries.
    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    /// Return the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the last entry, if any.
    pub fn last(&self) -> Option<&IndexEntry> {
        self.entries.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_index_needs_full() {
        let idx = CheckpointIndex::new();
        assert!(idx.needs_full());
        assert_eq!(idx.next_id(), 1);
    }

    #[test]
    fn register_increments_id() {
        let mut idx = CheckpointIndex::new();
        let id1 = idx.register(true, "hash1".to_string());
        let id2 = idx.register(false, "hash2".to_string());
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(idx.next_id(), 3);
    }

    #[test]
    fn last_full_id_tracks_correctly() {
        let mut idx = CheckpointIndex::new();
        idx.register(true, "a".to_string());
        idx.register(false, "b".to_string());
        idx.register(false, "c".to_string());
        assert_eq!(idx.last_full_id(), Some(1));
    }

    #[test]
    fn deltas_since_full_counts_correctly() {
        let mut idx = CheckpointIndex::new();
        idx.register(true, "a".to_string());
        idx.register(false, "b".to_string());
        idx.register(false, "c".to_string());
        assert_eq!(idx.deltas_since_full(), 2);
    }

    #[test]
    fn monotonic_ids() {
        let mut idx = CheckpointIndex::new();
        let mut prev = 0;
        for i in 0..10 {
            let id = idx.register(i % 5 == 0, format!("h{i}"));
            assert!(id > prev);
            prev = id;
        }
    }
}
