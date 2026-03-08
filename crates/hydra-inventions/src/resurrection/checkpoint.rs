//! Checkpoint — save/restore full state snapshots.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub type CheckpointId = String;

/// A full state checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub label: String,
    pub timestamp: String,
    pub state: HashMap<String, serde_json::Value>,
    pub parent_id: Option<CheckpointId>,
    pub hash: String,
    pub size_bytes: usize,
}

impl Checkpoint {
    /// Create a new checkpoint from state
    pub fn create(
        label: &str,
        state: HashMap<String, serde_json::Value>,
        parent: Option<&str>,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();
        let serialized = serde_json::to_string(&state).unwrap_or_default();
        let size_bytes = serialized.len();

        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        Self {
            id,
            label: label.into(),
            timestamp,
            state,
            parent_id: parent.map(String::from),
            hash,
            size_bytes,
        }
    }

    /// Compute incremental diff from another checkpoint
    pub fn diff(&self, other: &Checkpoint) -> CheckpointDiff {
        let mut added = HashMap::new();
        let mut modified = HashMap::new();
        let mut removed = Vec::new();

        for (key, value) in &self.state {
            match other.state.get(key) {
                None => {
                    added.insert(key.clone(), value.clone());
                }
                Some(old_val) if old_val != value => {
                    modified.insert(key.clone(), value.clone());
                }
                _ => {}
            }
        }

        for key in other.state.keys() {
            if !self.state.contains_key(key) {
                removed.push(key.clone());
            }
        }

        CheckpointDiff {
            added,
            modified,
            removed,
        }
    }
}

/// Incremental diff between checkpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointDiff {
    pub added: HashMap<String, serde_json::Value>,
    pub modified: HashMap<String, serde_json::Value>,
    pub removed: Vec<String>,
}

impl CheckpointDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }

    pub fn change_count(&self) -> usize {
        self.added.len() + self.modified.len() + self.removed.len()
    }
}

/// Store for checkpoints
pub struct CheckpointStore {
    checkpoints: parking_lot::RwLock<Vec<Checkpoint>>,
    max_checkpoints: usize,
}

impl CheckpointStore {
    pub fn new(max_checkpoints: usize) -> Self {
        Self {
            checkpoints: parking_lot::RwLock::new(Vec::new()),
            max_checkpoints,
        }
    }

    /// Save a checkpoint
    pub fn save(&self, checkpoint: Checkpoint) {
        let mut store = self.checkpoints.write();
        store.push(checkpoint);
        // Evict oldest if over limit
        while store.len() > self.max_checkpoints {
            store.remove(0);
        }
    }

    /// Restore a checkpoint by ID
    pub fn restore(&self, id: &str) -> Option<Checkpoint> {
        self.checkpoints.read().iter().find(|c| c.id == id).cloned()
    }

    /// Get the latest checkpoint
    pub fn latest(&self) -> Option<Checkpoint> {
        self.checkpoints.read().last().cloned()
    }

    /// List all checkpoint summaries
    pub fn list(&self) -> Vec<(CheckpointId, String, String)> {
        self.checkpoints
            .read()
            .iter()
            .map(|c| (c.id.clone(), c.label.clone(), c.timestamp.clone()))
            .collect()
    }

    /// Remove old checkpoints, keeping only the last N
    pub fn cleanup(&self, keep: usize) {
        let mut store = self.checkpoints.write();
        if store.len() > keep {
            let drain_count = store.len() - keep;
            store.drain(0..drain_count);
        }
    }

    pub fn count(&self) -> usize {
        self.checkpoints.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state(key: &str, val: &str) -> HashMap<String, serde_json::Value> {
        HashMap::from([(key.into(), serde_json::json!(val))])
    }

    #[test]
    fn test_checkpoint_save_restore() {
        let store = CheckpointStore::new(10);
        let state = sample_state("memory", "hello world");
        let cp = Checkpoint::create("test-1", state.clone(), None);
        let id = cp.id.clone();
        store.save(cp);

        let restored = store.restore(&id).unwrap();
        assert_eq!(restored.state["memory"], serde_json::json!("hello world"));
        assert_eq!(restored.label, "test-1");
    }

    #[test]
    fn test_incremental_diff() {
        let state_a = HashMap::from([
            ("a".into(), serde_json::json!(1)),
            ("b".into(), serde_json::json!(2)),
            ("c".into(), serde_json::json!(3)),
        ]);
        let state_b = HashMap::from([
            ("a".into(), serde_json::json!(1)),  // unchanged
            ("b".into(), serde_json::json!(99)), // modified
            ("d".into(), serde_json::json!(4)),  // added
                                                 // c removed
        ]);

        let cp_a = Checkpoint::create("a", state_a, None);
        let cp_b = Checkpoint::create("b", state_b, Some(&cp_a.id));
        let diff = cp_b.diff(&cp_a);

        assert_eq!(diff.added.len(), 1);
        assert!(diff.added.contains_key("d"));
        assert_eq!(diff.modified.len(), 1);
        assert!(diff.modified.contains_key("b"));
        assert_eq!(diff.removed, vec!["c"]);
    }

    #[test]
    fn test_state_serialization() {
        let state = HashMap::from([("key".into(), serde_json::json!({"nested": true}))]);
        let cp = Checkpoint::create("ser-test", state, None);
        let json = serde_json::to_string(&cp).unwrap();
        let restored: Checkpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.label, "ser-test");
        assert!(!restored.hash.is_empty());
        assert!(restored.size_bytes > 0);
    }

    #[test]
    fn test_checkpoint_cleanup() {
        let store = CheckpointStore::new(100);
        for i in 0..10 {
            store.save(Checkpoint::create(
                &format!("cp-{}", i),
                HashMap::new(),
                None,
            ));
        }
        assert_eq!(store.count(), 10);
        store.cleanup(3);
        assert_eq!(store.count(), 3);
    }
}
