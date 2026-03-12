//! PeerDiscovery — mDNS, manual config, bootstrap nodes.

use serde::{Deserialize, Serialize};

/// Discovery method configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMethod {
    /// mDNS for local network discovery
    Mdns,
    /// Manual peer list
    Manual(Vec<String>),
    /// Bootstrap nodes for initial discovery
    Bootstrap(Vec<String>),
}

/// A discovered peer endpoint (not yet connected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    pub endpoint: String,
    pub method: String,
    pub discovered_at: String,
}

/// Peer discovery service
pub struct PeerDiscovery {
    method: DiscoveryMethod,
    discovered: parking_lot::Mutex<Vec<DiscoveredPeer>>,
}

impl PeerDiscovery {
    pub fn new(method: DiscoveryMethod) -> Self {
        Self {
            method,
            discovered: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// Discover peers using configured method
    pub fn discover(&self) -> Vec<DiscoveredPeer> {
        match &self.method {
            DiscoveryMethod::Manual(endpoints) => {
                let now = chrono::Utc::now().to_rfc3339();
                let peers: Vec<_> = endpoints
                    .iter()
                    .map(|ep| DiscoveredPeer {
                        endpoint: ep.clone(),
                        method: "manual".into(),
                        discovered_at: now.clone(),
                    })
                    .collect();
                *self.discovered.lock() = peers.clone();
                peers
            }
            DiscoveryMethod::Bootstrap(nodes) => {
                let now = chrono::Utc::now().to_rfc3339();
                let peers: Vec<_> = nodes
                    .iter()
                    .map(|ep| DiscoveredPeer {
                        endpoint: ep.clone(),
                        method: "bootstrap".into(),
                        discovered_at: now.clone(),
                    })
                    .collect();
                *self.discovered.lock() = peers.clone();
                peers
            }
            DiscoveryMethod::Mdns => {
                // mDNS would use async network scanning in production
                // Returns empty in non-async context
                Vec::new()
            }
        }
    }

    /// Get previously discovered peers
    pub fn cached(&self) -> Vec<DiscoveredPeer> {
        self.discovered.lock().clone()
    }

    /// Add a manually discovered peer
    pub fn add_manual(&self, endpoint: &str) {
        self.discovered.lock().push(DiscoveredPeer {
            endpoint: endpoint.into(),
            method: "manual_add".into(),
            discovered_at: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Number of discovered peers
    pub fn count(&self) -> usize {
        self.discovered.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_manual() {
        let discovery = PeerDiscovery::new(DiscoveryMethod::Manual(vec![
            "192.168.1.10:9000".into(),
            "192.168.1.20:9000".into(),
        ]));

        let peers = discovery.discover();
        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].endpoint, "192.168.1.10:9000");
        assert_eq!(peers[0].method, "manual");
    }

    #[test]
    fn test_discovery_bootstrap() {
        let discovery = PeerDiscovery::new(DiscoveryMethod::Bootstrap(vec![
            "bootstrap.hydra.local:9000".into(),
        ]));

        let peers = discovery.discover();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].method, "bootstrap");
    }

    #[test]
    fn test_discovery_mdns_returns_empty() {
        let discovery = PeerDiscovery::new(DiscoveryMethod::Mdns);
        let peers = discovery.discover();
        assert!(peers.is_empty()); // No actual mDNS in tests
    }

    #[test]
    fn test_discovery_cached_empty_initially() {
        let discovery = PeerDiscovery::new(DiscoveryMethod::Manual(vec!["1.2.3.4:9000".into()]));
        assert!(discovery.cached().is_empty());
    }

    #[test]
    fn test_discovery_cached_after_discover() {
        let discovery = PeerDiscovery::new(DiscoveryMethod::Manual(vec!["10.0.0.1:9000".into()]));
        discovery.discover();
        let cached = discovery.cached();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].endpoint, "10.0.0.1:9000");
    }

    #[test]
    fn test_discovery_count() {
        let discovery = PeerDiscovery::new(DiscoveryMethod::Manual(vec![
            "a:9000".into(),
            "b:9000".into(),
            "c:9000".into(),
        ]));
        discovery.discover();
        assert_eq!(discovery.count(), 3);
    }

    #[test]
    fn test_discovery_multiple_manual_adds() {
        let discovery = PeerDiscovery::new(DiscoveryMethod::Mdns);
        discovery.add_manual("10.0.0.1:9000");
        discovery.add_manual("10.0.0.2:9000");
        assert_eq!(discovery.count(), 2);
    }

    #[test]
    fn test_discovery_method_serialization() {
        let method = DiscoveryMethod::Mdns;
        let json = serde_json::to_string(&method).unwrap();
        let restored: DiscoveryMethod = serde_json::from_str(&json).unwrap();
        assert!(matches!(restored, DiscoveryMethod::Mdns));
    }

    #[test]
    fn test_discovered_peer_serialization() {
        let peer = DiscoveredPeer {
            endpoint: "10.0.0.1:9000".into(),
            method: "manual".into(),
            discovered_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&peer).unwrap();
        let restored: DiscoveredPeer = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.endpoint, "10.0.0.1:9000");
    }

    #[test]
    fn test_discovery_add_manual() {
        let discovery = PeerDiscovery::new(DiscoveryMethod::Mdns);
        assert_eq!(discovery.count(), 0);

        discovery.add_manual("10.0.0.5:9000");
        assert_eq!(discovery.count(), 1);

        let cached = discovery.cached();
        assert_eq!(cached[0].endpoint, "10.0.0.5:9000");
        assert_eq!(cached[0].method, "manual_add");
    }
}
