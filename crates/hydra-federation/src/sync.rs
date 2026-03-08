//! SyncProtocol — CRDT-based state sync with conflict resolution.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("version conflict: local={local}, remote={remote}")]
    VersionConflict { local: u64, remote: u64 },
    #[error("merge failed: {0}")]
    MergeFailed(String),
}

/// A versioned sync entry (LWW register)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub version: u64,
    pub timestamp: String,
    pub origin_peer: String,
}

/// Sync state — a map of versioned entries
#[derive(Debug, Clone, Default)]
pub struct SyncState {
    entries: HashMap<String, SyncEntry>,
    version: u64,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    /// Last-write-wins based on timestamp
    LastWriteWins,
    /// Higher version wins
    HigherVersion,
    /// Keep local on conflict
    KeepLocal,
    /// Keep remote on conflict
    KeepRemote,
}

/// Result of a sync operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncReport {
    pub incoming_applied: u32,
    pub outgoing_sent: u32,
    pub conflicts_resolved: u32,
    pub strategy_used: String,
}

/// CRDT-based sync protocol
pub struct SyncProtocol {
    state: parking_lot::RwLock<SyncState>,
    strategy: ConflictStrategy,
}

impl SyncProtocol {
    pub fn new(strategy: ConflictStrategy) -> Self {
        Self {
            state: parking_lot::RwLock::new(SyncState::default()),
            strategy,
        }
    }

    /// Apply a local change
    pub fn local_put(&self, key: &str, value: serde_json::Value, origin: &str) {
        let mut state = self.state.write();
        state.version += 1;
        let version = state.version;
        state.entries.insert(
            key.into(),
            SyncEntry {
                key: key.into(),
                value,
                version,
                timestamp: chrono::Utc::now().to_rfc3339(),
                origin_peer: origin.into(),
            },
        );
    }

    /// Get current value
    pub fn get(&self, key: &str) -> Option<SyncEntry> {
        self.state.read().entries.get(key).cloned()
    }

    /// Get all entries since a version
    pub fn changes_since(&self, since_version: u64) -> Vec<SyncEntry> {
        self.state
            .read()
            .entries
            .values()
            .filter(|e| e.version > since_version)
            .cloned()
            .collect()
    }

    /// Merge remote entries into local state
    pub fn merge(&self, remote_entries: Vec<SyncEntry>) -> SyncReport {
        let mut state = self.state.write();
        let mut report = SyncReport {
            strategy_used: format!("{:?}", self.strategy),
            ..Default::default()
        };

        for remote in remote_entries {
            if let Some(local) = state.entries.get(&remote.key) {
                // Conflict — resolve based on strategy
                let keep_remote = match self.strategy {
                    ConflictStrategy::LastWriteWins => remote.timestamp > local.timestamp,
                    ConflictStrategy::HigherVersion => remote.version > local.version,
                    ConflictStrategy::KeepLocal => false,
                    ConflictStrategy::KeepRemote => true,
                };

                if keep_remote {
                    state.entries.insert(remote.key.clone(), remote);
                }
                report.conflicts_resolved += 1;
            } else {
                // No conflict — apply directly
                state.entries.insert(remote.key.clone(), remote);
                report.incoming_applied += 1;
            }
        }

        // Update local version to max
        if let Some(max_v) = state.entries.values().map(|e| e.version).max() {
            if max_v > state.version {
                state.version = max_v;
            }
        }

        report
    }

    /// Current version
    pub fn version(&self) -> u64 {
        self.state.read().version
    }

    /// Number of entries
    pub fn entry_count(&self) -> usize {
        self.state.read().entries.len()
    }
}

impl Default for SyncProtocol {
    fn default() -> Self {
        Self::new(ConflictStrategy::LastWriteWins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_local_put_and_get() {
        let sync = SyncProtocol::default();
        sync.local_put("key1", serde_json::json!("value1"), "local");

        let entry = sync.get("key1").unwrap();
        assert_eq!(entry.value, serde_json::json!("value1"));
        assert_eq!(sync.version(), 1);
    }

    #[test]
    fn test_sync_merge_no_conflict() {
        let sync = SyncProtocol::default();

        let remote = vec![SyncEntry {
            key: "remote_key".into(),
            value: serde_json::json!("remote_value"),
            version: 5,
            timestamp: chrono::Utc::now().to_rfc3339(),
            origin_peer: "peer-b".into(),
        }];

        let report = sync.merge(remote);
        assert_eq!(report.incoming_applied, 1);
        assert_eq!(report.conflicts_resolved, 0);
        assert!(sync.get("remote_key").is_some());
    }

    #[test]
    fn test_sync_merge_conflict_lww() {
        let sync = SyncProtocol::new(ConflictStrategy::LastWriteWins);

        // Local entry with old timestamp
        sync.local_put("key", serde_json::json!("local"), "local");

        // Remote entry with newer timestamp
        let remote = vec![SyncEntry {
            key: "key".into(),
            value: serde_json::json!("remote"),
            version: 10,
            timestamp: "2099-01-01T00:00:00Z".into(), // Future timestamp wins
            origin_peer: "peer-b".into(),
        }];

        let report = sync.merge(remote);
        assert_eq!(report.conflicts_resolved, 1);
        assert_eq!(sync.get("key").unwrap().value, serde_json::json!("remote"));
    }

    #[test]
    fn test_sync_merge_keep_local() {
        let sync = SyncProtocol::new(ConflictStrategy::KeepLocal);
        sync.local_put("key", serde_json::json!("local"), "local");

        let remote = vec![SyncEntry {
            key: "key".into(),
            value: serde_json::json!("remote"),
            version: 100,
            timestamp: "2099-01-01T00:00:00Z".into(),
            origin_peer: "peer-b".into(),
        }];

        sync.merge(remote);
        assert_eq!(sync.get("key").unwrap().value, serde_json::json!("local"));
    }

    #[test]
    fn test_sync_changes_since() {
        let sync = SyncProtocol::default();
        sync.local_put("a", serde_json::json!(1), "local");
        sync.local_put("b", serde_json::json!(2), "local");
        sync.local_put("c", serde_json::json!(3), "local");

        let changes = sync.changes_since(1);
        assert_eq!(changes.len(), 2); // b and c (version > 1)
    }
}
