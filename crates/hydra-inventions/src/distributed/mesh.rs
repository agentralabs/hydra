//! DistributedHydra — P2P mesh for distributed self.
//!
//! Enables multiple Hydra instances to form a mesh network,
//! share capabilities, and coordinate work.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub type PeerId = String;

/// Information about a peer in the mesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: PeerId,
    pub name: String,
    pub address: String,
    pub capabilities: Vec<String>,
    pub status: PeerStatus,
    pub last_seen: String,
    pub latency_ms: Option<u64>,
}

/// Status of a peer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    Connected,
    Disconnected,
    Syncing,
    Degraded,
}

/// A capability request to be delegated to a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRequest {
    pub id: String,
    pub capability: String,
    pub payload: serde_json::Value,
    pub preferred_peer: Option<PeerId>,
    pub timeout_ms: u64,
}

/// Result of a delegation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResult {
    pub request_id: String,
    pub peer_id: PeerId,
    pub success: bool,
    pub result: serde_json::Value,
    pub latency_ms: u64,
}

/// Distributed Hydra mesh coordinator
pub struct DistributedHydra {
    local_id: PeerId,
    peers: parking_lot::RwLock<HashMap<PeerId, PeerInfo>>,
    delegations: parking_lot::RwLock<Vec<DelegationResult>>,
    max_peers: usize,
}

impl DistributedHydra {
    pub fn new(local_id: &str, max_peers: usize) -> Self {
        Self {
            local_id: local_id.into(),
            peers: parking_lot::RwLock::new(HashMap::new()),
            delegations: parking_lot::RwLock::new(Vec::new()),
            max_peers,
        }
    }

    /// Get local peer ID
    pub fn local_id(&self) -> &str {
        &self.local_id
    }

    /// Register a peer in the mesh
    pub fn register_peer(&self, info: PeerInfo) -> bool {
        let mut peers = self.peers.write();
        if peers.len() >= self.max_peers && !peers.contains_key(&info.id) {
            return false;
        }
        peers.insert(info.id.clone(), info);
        true
    }

    /// Remove a peer from the mesh
    pub fn remove_peer(&self, peer_id: &str) -> bool {
        self.peers.write().remove(peer_id).is_some()
    }

    /// Update peer status
    pub fn update_status(&self, peer_id: &str, status: PeerStatus) -> bool {
        if let Some(peer) = self.peers.write().get_mut(peer_id) {
            peer.status = status;
            peer.last_seen = chrono::Utc::now().to_rfc3339();
            true
        } else {
            false
        }
    }

    /// Find peers that have a specific capability
    pub fn find_capable_peers(&self, capability: &str) -> Vec<PeerInfo> {
        self.peers
            .read()
            .values()
            .filter(|p| {
                p.status == PeerStatus::Connected
                    && p.capabilities.iter().any(|c| c == capability)
            })
            .cloned()
            .collect()
    }

    /// Delegate a capability request to the best available peer
    pub fn delegate(&self, request: DelegationRequest) -> Option<DelegationResult> {
        let peers = self.find_capable_peers(&request.capability);

        // Select best peer: prefer specified, then lowest latency
        let peer = if let Some(preferred) = &request.preferred_peer {
            peers.iter().find(|p| &p.id == preferred).or(peers.first())
        } else {
            peers
                .iter()
                .min_by_key(|p| p.latency_ms.unwrap_or(u64::MAX))
        };

        let peer = peer?;

        // Simulate delegation (real impl would make network call)
        let result = DelegationResult {
            request_id: request.id,
            peer_id: peer.id.clone(),
            success: true,
            result: serde_json::json!({"delegated_to": peer.name, "status": "completed"}),
            latency_ms: peer.latency_ms.unwrap_or(100),
        };

        self.delegations.write().push(result.clone());
        Some(result)
    }

    /// Get all connected peers
    pub fn connected_peers(&self) -> Vec<PeerInfo> {
        self.peers
            .read()
            .values()
            .filter(|p| p.status == PeerStatus::Connected)
            .cloned()
            .collect()
    }

    /// Get mesh statistics
    pub fn mesh_stats(&self) -> MeshStats {
        let peers = self.peers.read();
        let connected = peers
            .values()
            .filter(|p| p.status == PeerStatus::Connected)
            .count();
        let total_capabilities: Vec<String> = peers
            .values()
            .flat_map(|p| p.capabilities.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        MeshStats {
            total_peers: peers.len(),
            connected_peers: connected,
            unique_capabilities: total_capabilities.len(),
            total_delegations: self.delegations.read().len(),
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }
}

/// Mesh network statistics
#[derive(Debug, Clone)]
pub struct MeshStats {
    pub total_peers: usize,
    pub connected_peers: usize,
    pub unique_capabilities: usize,
    pub total_delegations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer(id: &str, caps: Vec<&str>) -> PeerInfo {
        PeerInfo {
            id: id.into(),
            name: format!("peer-{}", id),
            address: format!("127.0.0.1:{}", 8000 + id.len()),
            capabilities: caps.into_iter().map(String::from).collect(),
            status: PeerStatus::Connected,
            last_seen: chrono::Utc::now().to_rfc3339(),
            latency_ms: Some(50),
        }
    }

    #[test]
    fn test_peer_registration() {
        let mesh = DistributedHydra::new("local", 10);
        let peer = make_peer("alpha", vec!["memory", "vision"]);
        assert!(mesh.register_peer(peer));
        assert_eq!(mesh.peer_count(), 1);
    }

    #[test]
    fn test_capability_discovery() {
        let mesh = DistributedHydra::new("local", 10);
        mesh.register_peer(make_peer("alpha", vec!["memory", "vision"]));
        mesh.register_peer(make_peer("beta", vec!["codebase", "memory"]));
        mesh.register_peer(make_peer("gamma", vec!["vision"]));

        let memory_peers = mesh.find_capable_peers("memory");
        assert_eq!(memory_peers.len(), 2);

        let vision_peers = mesh.find_capable_peers("vision");
        assert_eq!(vision_peers.len(), 2);

        let codebase_peers = mesh.find_capable_peers("codebase");
        assert_eq!(codebase_peers.len(), 1);
    }

    #[test]
    fn test_delegation() {
        let mesh = DistributedHydra::new("local", 10);
        mesh.register_peer(make_peer("alpha", vec!["memory"]));

        let request = DelegationRequest {
            id: "req-1".into(),
            capability: "memory".into(),
            payload: serde_json::json!({"query": "test"}),
            preferred_peer: None,
            timeout_ms: 5000,
        };

        let result = mesh.delegate(request).unwrap();
        assert!(result.success);
        assert_eq!(result.peer_id, "alpha");
    }

    #[test]
    fn test_max_peers_limit() {
        let mesh = DistributedHydra::new("local", 2);
        assert!(mesh.register_peer(make_peer("a", vec![])));
        assert!(mesh.register_peer(make_peer("b", vec![])));
        assert!(!mesh.register_peer(make_peer("c", vec![])));
    }

    #[test]
    fn test_mesh_stats() {
        let mesh = DistributedHydra::new("local", 10);
        mesh.register_peer(make_peer("alpha", vec!["memory", "vision"]));
        mesh.register_peer(make_peer("beta", vec!["codebase", "memory"]));

        let stats = mesh.mesh_stats();
        assert_eq!(stats.total_peers, 2);
        assert_eq!(stats.connected_peers, 2);
        assert_eq!(stats.unique_capabilities, 3);
    }
}
