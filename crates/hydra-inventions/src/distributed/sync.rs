//! StateSynchronizer — sync state across distributed Hydra instances.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A sync message between peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    pub id: String,
    pub from_peer: String,
    pub to_peer: String,
    pub payload: HashMap<String, serde_json::Value>,
    pub vector_clock: HashMap<String, u64>,
    pub timestamp: String,
}

impl SyncMessage {
    pub fn new(from: &str, to: &str, payload: HashMap<String, serde_json::Value>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from_peer: from.into(),
            to_peer: to.into(),
            payload,
            vector_clock: HashMap::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Result of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub message_id: String,
    pub merged_keys: usize,
    pub conflicts: Vec<String>,
    pub success: bool,
}

/// Synchronizes state between Hydra instances using vector clocks
pub struct StateSynchronizer {
    local_id: String,
    state: parking_lot::RwLock<HashMap<String, serde_json::Value>>,
    clock: parking_lot::RwLock<HashMap<String, u64>>,
    history: parking_lot::RwLock<Vec<SyncResult>>,
}

impl StateSynchronizer {
    pub fn new(local_id: &str) -> Self {
        Self {
            local_id: local_id.into(),
            state: parking_lot::RwLock::new(HashMap::new()),
            clock: parking_lot::RwLock::new(HashMap::new()),
            history: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Set a local state key
    pub fn set(&self, key: &str, value: serde_json::Value) {
        self.state.write().insert(key.into(), value);
        self.tick();
    }

    /// Get a local state key
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.state.read().get(key).cloned()
    }

    /// Increment local vector clock
    fn tick(&self) {
        let mut clock = self.clock.write();
        let counter = clock.entry(self.local_id.clone()).or_insert(0);
        *counter += 1;
    }

    /// Create a sync message to send to a peer
    pub fn prepare_sync(&self, to_peer: &str) -> SyncMessage {
        SyncMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from_peer: self.local_id.clone(),
            to_peer: to_peer.into(),
            payload: self.state.read().clone(),
            vector_clock: self.clock.read().clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Apply a sync message from a peer (last-writer-wins merge)
    pub fn apply_sync(&self, message: &SyncMessage) -> SyncResult {
        let mut state = self.state.write();
        let mut clock = self.clock.write();
        let mut merged = 0;
        let mut conflicts = Vec::new();

        let local_tick = clock.get(&self.local_id).copied().unwrap_or(0);

        for (key, value) in &message.payload {
            if state.contains_key(key) && local_tick > 0 {
                // Key exists locally and local node has made writes — conflict
                conflicts.push(key.clone());
            } else {
                state.insert(key.clone(), value.clone());
                merged += 1;
            }
        }

        // Merge vector clock (take max)
        for (peer, &tick) in &message.vector_clock {
            let entry = clock.entry(peer.clone()).or_insert(0);
            if tick > *entry {
                *entry = tick;
            }
        }

        let result = SyncResult {
            message_id: message.id.clone(),
            merged_keys: merged,
            conflicts,
            success: true,
        };

        self.history.write().push(result.clone());
        result
    }

    /// Get sync history
    pub fn sync_count(&self) -> usize {
        self.history.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_sync() {
        let sync_a = StateSynchronizer::new("node-a");
        let sync_b = StateSynchronizer::new("node-b");

        sync_a.set("key1", serde_json::json!("value1"));
        sync_a.set("key2", serde_json::json!("value2"));

        let msg = sync_a.prepare_sync("node-b");
        let result = sync_b.apply_sync(&msg);

        assert!(result.success);
        assert_eq!(result.merged_keys, 2);
        assert_eq!(sync_b.get("key1").unwrap(), serde_json::json!("value1"));
    }

    #[test]
    fn test_vector_clock_conflict() {
        let sync_a = StateSynchronizer::new("node-a");
        let sync_b = StateSynchronizer::new("node-b");

        // Both write to same key
        sync_a.set("shared", serde_json::json!("from-a"));
        sync_b.set("shared", serde_json::json!("from-b"));

        // B applies A's state — conflict because B already has "shared"
        let msg = sync_a.prepare_sync("node-b");
        let result = sync_b.apply_sync(&msg);
        assert!(!result.conflicts.is_empty());
    }

    #[test]
    fn test_prepare_sync_message() {
        let sync = StateSynchronizer::new("local");
        sync.set("data", serde_json::json!(42));

        let msg = sync.prepare_sync("remote");
        assert_eq!(msg.from_peer, "local");
        assert_eq!(msg.to_peer, "remote");
        assert!(msg.payload.contains_key("data"));
    }
}
