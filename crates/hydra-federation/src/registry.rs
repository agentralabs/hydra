//! PeerRegistry — known peers, trust scoring.

use std::collections::HashMap;

use parking_lot::RwLock;
use thiserror::Error;

use crate::peer::{PeerId, PeerInfo, TrustLevel};

#[derive(Debug, Error)]
pub enum PeerRegistryError {
    #[error("peer not found: {0}")]
    NotFound(String),
    #[error("peer already registered: {0}")]
    AlreadyExists(String),
}

/// Registry of known peers with trust management
pub struct PeerRegistry {
    peers: RwLock<HashMap<PeerId, PeerInfo>>,
}

impl PeerRegistry {
    pub fn new() -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
        }
    }

    /// Add or update a peer
    pub fn register(&self, peer: PeerInfo) {
        self.peers.write().insert(peer.id.clone(), peer);
    }

    /// Get peer by ID
    pub fn get(&self, id: &str) -> Option<PeerInfo> {
        self.peers.read().get(id).cloned()
    }

    /// Remove a peer
    pub fn remove(&self, id: &str) -> bool {
        self.peers.write().remove(id).is_some()
    }

    /// Set trust level for a peer
    pub fn set_trust(&self, id: &str, level: TrustLevel) -> Result<(), PeerRegistryError> {
        let mut peers = self.peers.write();
        let peer = peers
            .get_mut(id)
            .ok_or_else(|| PeerRegistryError::NotFound(id.into()))?;
        peer.trust_level = level;
        Ok(())
    }

    /// Get all peers at or above a trust level
    pub fn peers_at_trust(&self, min_trust: TrustLevel) -> Vec<PeerInfo> {
        self.peers
            .read()
            .values()
            .filter(|p| p.trust_level >= min_trust)
            .cloned()
            .collect()
    }

    /// Get peers capable of handling a specific requirement
    pub fn capable_peers(&self, capability: &str) -> Vec<PeerInfo> {
        self.peers
            .read()
            .values()
            .filter(|p| p.has_capability(capability) && p.allows_delegation())
            .cloned()
            .collect()
    }

    /// Get peers with available capacity
    pub fn available_peers(&self) -> Vec<PeerInfo> {
        self.peers
            .read()
            .values()
            .filter(|p| p.has_capacity() && p.allows_delegation())
            .cloned()
            .collect()
    }

    /// Update a peer's active task count
    pub fn update_task_count(&self, id: &str, count: u32) {
        if let Some(peer) = self.peers.write().get_mut(id) {
            peer.active_tasks = count;
        }
    }

    /// Update last seen timestamp
    pub fn touch(&self, id: &str) {
        if let Some(peer) = self.peers.write().get_mut(id) {
            peer.last_seen = chrono::Utc::now().to_rfc3339();
        }
    }

    /// Number of registered peers
    pub fn count(&self) -> usize {
        self.peers.read().len()
    }

    /// List all peers
    pub fn list(&self) -> Vec<PeerInfo> {
        self.peers.read().values().cloned().collect()
    }
}

impl Default for PeerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::{FederationType, PeerCapabilities};

    fn make_peer(id: &str, trust: TrustLevel) -> PeerInfo {
        PeerInfo {
            id: id.into(),
            name: format!("peer-{}", id),
            endpoint: format!("{}:9000", id),
            version: "0.1.0".into(),
            capabilities: PeerCapabilities {
                sisters: vec!["memory".into(), "codebase".into()],
                skills: vec![],
                max_concurrent_tasks: 4,
                available_memory_mb: 1024,
                federation_types: vec![FederationType::Personal],
            },
            trust_level: trust,
            federation_type: FederationType::Personal,
            last_seen: chrono::Utc::now().to_rfc3339(),
            active_tasks: 0,
        }
    }

    #[test]
    fn test_registry_add_peer() {
        let registry = PeerRegistry::new();
        registry.register(make_peer("a", TrustLevel::Trusted));
        assert_eq!(registry.count(), 1);
        assert!(registry.get("a").is_some());
    }

    #[test]
    fn test_registry_trust_levels() {
        let registry = PeerRegistry::new();
        registry.register(make_peer("owner", TrustLevel::Owner));
        registry.register(make_peer("trusted", TrustLevel::Trusted));
        registry.register(make_peer("known", TrustLevel::Known));
        registry.register(make_peer("unknown", TrustLevel::Unknown));

        let trusted_plus = registry.peers_at_trust(TrustLevel::Trusted);
        assert_eq!(trusted_plus.len(), 2); // Owner + Trusted

        let known_plus = registry.peers_at_trust(TrustLevel::Known);
        assert_eq!(known_plus.len(), 3); // Owner + Trusted + Known
    }

    #[test]
    fn test_registry_capable_peers() {
        let registry = PeerRegistry::new();
        registry.register(make_peer("a", TrustLevel::Trusted));
        registry.register(make_peer("b", TrustLevel::Known)); // Known can't delegate

        let capable = registry.capable_peers("memory");
        assert_eq!(capable.len(), 1); // Only trusted peer
        assert_eq!(capable[0].id, "a");
    }

    #[test]
    fn test_registry_remove() {
        let registry = PeerRegistry::new();
        registry.register(make_peer("a", TrustLevel::Trusted));
        assert!(registry.remove("a"));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_remove_nonexistent() {
        let registry = PeerRegistry::new();
        assert!(!registry.remove("nonexistent"));
    }

    #[test]
    fn test_registry_list() {
        let registry = PeerRegistry::new();
        registry.register(make_peer("a", TrustLevel::Trusted));
        registry.register(make_peer("b", TrustLevel::Owner));
        let list = registry.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_registry_update_task_count() {
        let registry = PeerRegistry::new();
        registry.register(make_peer("a", TrustLevel::Trusted));
        registry.update_task_count("a", 5);
        let peer = registry.get("a").unwrap();
        assert_eq!(peer.active_tasks, 5);
    }

    #[test]
    fn test_registry_touch() {
        let registry = PeerRegistry::new();
        registry.register(make_peer("a", TrustLevel::Trusted));
        let before = registry.get("a").unwrap().last_seen.clone();
        std::thread::sleep(std::time::Duration::from_millis(10));
        registry.touch("a");
        let after = registry.get("a").unwrap().last_seen;
        assert_ne!(before, after);
    }

    #[test]
    fn test_registry_available_peers() {
        let registry = PeerRegistry::new();
        registry.register(make_peer("a", TrustLevel::Trusted)); // capacity, delegation OK
        registry.register(make_peer("b", TrustLevel::Known));   // no delegation
        let available = registry.available_peers();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id, "a");
    }

    #[test]
    fn test_registry_set_trust_nonexistent() {
        let registry = PeerRegistry::new();
        let result = registry.set_trust("nonexistent", TrustLevel::Owner);
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_default() {
        let registry = PeerRegistry::default();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_set_trust() {
        let registry = PeerRegistry::new();
        registry.register(make_peer("a", TrustLevel::Unknown));
        assert_eq!(registry.get("a").unwrap().trust_level, TrustLevel::Unknown);

        registry.set_trust("a", TrustLevel::Trusted).unwrap();
        assert_eq!(registry.get("a").unwrap().trust_level, TrustLevel::Trusted);
    }
}
