//! PeerId, PeerInfo, PeerCapabilities.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Unique peer identifier (SHA-256 of public key)
pub type PeerId = String;

/// Trust levels for federation peers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Discovery only, no delegation
    Unknown,
    /// Read-only skill sharing, limited delegation
    Known,
    /// Full skill sharing, task delegation
    Trusted,
    /// Same user, full sync
    Owner,
}

/// Federation type determines sync scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FederationType {
    /// Same user, multiple devices — full sync
    Personal,
    /// Multiple users, shared project — partial sync
    Team,
    /// Anonymous pattern sharing — patterns only
    Collective,
}

/// Peer capabilities advertised during hello
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerCapabilities {
    pub sisters: Vec<String>,
    pub skills: Vec<String>,
    pub max_concurrent_tasks: u32,
    pub available_memory_mb: u64,
    pub federation_types: Vec<FederationType>,
}

impl Default for PeerCapabilities {
    fn default() -> Self {
        Self {
            sisters: Vec::new(),
            skills: Vec::new(),
            max_concurrent_tasks: 4,
            available_memory_mb: 512,
            federation_types: vec![FederationType::Personal],
        }
    }
}

/// Information about a known peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: PeerId,
    pub name: String,
    pub endpoint: String,
    pub version: String,
    pub capabilities: PeerCapabilities,
    pub trust_level: TrustLevel,
    pub federation_type: FederationType,
    pub last_seen: String,
    pub active_tasks: u32,
}

impl PeerInfo {
    /// Check if this peer can handle a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.sisters.iter().any(|s| s == capability)
            || self.capabilities.skills.iter().any(|s| s == capability)
    }

    /// Check if peer has capacity for more tasks
    pub fn has_capacity(&self) -> bool {
        self.active_tasks < self.capabilities.max_concurrent_tasks
    }

    /// Check if peer allows delegation at its trust level
    pub fn allows_delegation(&self) -> bool {
        self.trust_level >= TrustLevel::Trusted
    }

    /// Check if peer allows skill sharing
    pub fn allows_skill_sharing(&self) -> bool {
        self.trust_level >= TrustLevel::Known
    }
}

/// Generate a peer ID from a name and secret
pub fn generate_peer_id(name: &str, secret: &str) -> PeerId {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(b":");
    hasher.update(secret.as_bytes());
    format!("peer-{}", hex::encode(&hasher.finalize()[..8]))
}

/// Hex encoding (minimal, no dependency)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_id_generation() {
        let id1 = generate_peer_id("hydra-laptop", "secret123");
        let id2 = generate_peer_id("hydra-laptop", "secret123");
        let id3 = generate_peer_id("hydra-server", "secret123");

        assert_eq!(id1, id2); // Deterministic
        assert_ne!(id1, id3); // Different name = different ID
        assert!(id1.starts_with("peer-"));
    }

    #[test]
    fn test_peer_capabilities() {
        let peer = PeerInfo {
            id: "peer-abc".into(),
            name: "test".into(),
            endpoint: "localhost:9000".into(),
            version: "0.1.0".into(),
            capabilities: PeerCapabilities {
                sisters: vec!["memory".into(), "vision".into()],
                skills: vec!["file_read".into()],
                max_concurrent_tasks: 4,
                available_memory_mb: 1024,
                federation_types: vec![FederationType::Personal],
            },
            trust_level: TrustLevel::Trusted,
            federation_type: FederationType::Personal,
            last_seen: chrono::Utc::now().to_rfc3339(),
            active_tasks: 1,
        };

        assert!(peer.has_capability("memory"));
        assert!(peer.has_capability("file_read"));
        assert!(!peer.has_capability("codebase"));
        assert!(peer.has_capacity());
        assert!(peer.allows_delegation());
        assert!(peer.allows_skill_sharing());
    }

    #[test]
    fn test_trust_level_ordering() {
        assert!(TrustLevel::Owner > TrustLevel::Trusted);
        assert!(TrustLevel::Trusted > TrustLevel::Known);
        assert!(TrustLevel::Known > TrustLevel::Unknown);
    }

    #[test]
    fn test_known_peer_can_share_but_not_delegate() {
        let peer = PeerInfo {
            id: "peer-k".into(),
            name: "known-peer".into(),
            endpoint: "localhost:9000".into(),
            version: "0.1.0".into(),
            capabilities: PeerCapabilities::default(),
            trust_level: TrustLevel::Known,
            federation_type: FederationType::Team,
            last_seen: chrono::Utc::now().to_rfc3339(),
            active_tasks: 0,
        };
        assert!(peer.allows_skill_sharing());
        assert!(!peer.allows_delegation());
    }

    #[test]
    fn test_peer_at_capacity() {
        let peer = PeerInfo {
            id: "peer-full".into(),
            name: "full".into(),
            endpoint: "localhost:9000".into(),
            version: "0.1.0".into(),
            capabilities: PeerCapabilities {
                max_concurrent_tasks: 2,
                ..PeerCapabilities::default()
            },
            trust_level: TrustLevel::Trusted,
            federation_type: FederationType::Personal,
            last_seen: chrono::Utc::now().to_rfc3339(),
            active_tasks: 2,
        };
        assert!(!peer.has_capacity());
    }

    #[test]
    fn test_peer_has_capacity_below_max() {
        let peer = PeerInfo {
            id: "peer-ok".into(),
            name: "ok".into(),
            endpoint: "localhost:9000".into(),
            version: "0.1.0".into(),
            capabilities: PeerCapabilities {
                max_concurrent_tasks: 4,
                ..PeerCapabilities::default()
            },
            trust_level: TrustLevel::Trusted,
            federation_type: FederationType::Personal,
            last_seen: chrono::Utc::now().to_rfc3339(),
            active_tasks: 3,
        };
        assert!(peer.has_capacity());
    }

    #[test]
    fn test_peer_capabilities_default() {
        let caps = PeerCapabilities::default();
        assert!(caps.sisters.is_empty());
        assert!(caps.skills.is_empty());
        assert_eq!(caps.max_concurrent_tasks, 4);
        assert_eq!(caps.available_memory_mb, 512);
        assert_eq!(caps.federation_types, vec![FederationType::Personal]);
    }

    #[test]
    fn test_peer_id_deterministic() {
        let id1 = generate_peer_id("same", "same");
        let id2 = generate_peer_id("same", "same");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_peer_id_different_secret() {
        let id1 = generate_peer_id("host", "secret1");
        let id2 = generate_peer_id("host", "secret2");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_peer_info_serialization() {
        let peer = PeerInfo {
            id: "peer-ser".into(),
            name: "serializable".into(),
            endpoint: "localhost:9000".into(),
            version: "0.1.0".into(),
            capabilities: PeerCapabilities::default(),
            trust_level: TrustLevel::Owner,
            federation_type: FederationType::Personal,
            last_seen: "2026-01-01T00:00:00Z".into(),
            active_tasks: 0,
        };
        let json = serde_json::to_string(&peer).unwrap();
        let restored: PeerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "peer-ser");
        assert_eq!(restored.trust_level, TrustLevel::Owner);
    }

    #[test]
    fn test_federation_type_serialization() {
        let json = serde_json::to_string(&FederationType::Personal).unwrap();
        assert_eq!(json, "\"Personal\"");
        let json = serde_json::to_string(&FederationType::Team).unwrap();
        assert_eq!(json, "\"Team\"");
        let json = serde_json::to_string(&FederationType::Collective).unwrap();
        assert_eq!(json, "\"Collective\"");
    }

    #[test]
    fn test_has_capability_in_skills() {
        let peer = PeerInfo {
            id: "p".into(),
            name: "p".into(),
            endpoint: "l".into(),
            version: "0.1.0".into(),
            capabilities: PeerCapabilities {
                sisters: vec![],
                skills: vec!["special_skill".into()],
                ..PeerCapabilities::default()
            },
            trust_level: TrustLevel::Trusted,
            federation_type: FederationType::Personal,
            last_seen: "now".into(),
            active_tasks: 0,
        };
        assert!(peer.has_capability("special_skill"));
        assert!(!peer.has_capability("memory"));
    }

    #[test]
    fn test_unknown_peer_restrictions() {
        let peer = PeerInfo {
            id: "peer-xyz".into(),
            name: "stranger".into(),
            endpoint: "1.2.3.4:9000".into(),
            version: "0.1.0".into(),
            capabilities: PeerCapabilities::default(),
            trust_level: TrustLevel::Unknown,
            federation_type: FederationType::Collective,
            last_seen: chrono::Utc::now().to_rfc3339(),
            active_tasks: 0,
        };

        assert!(!peer.allows_delegation());
        assert!(!peer.allows_skill_sharing());
    }
}
