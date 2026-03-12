//! Federation panel — peer display, shared skills, sync status.

use serde::{Deserialize, Serialize};

/// Health status of a federation peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerHealth {
    Healthy,
    Degraded,
    Disconnected,
}

impl PeerHealth {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Degraded => "Degraded",
            Self::Disconnected => "Disconnected",
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Healthy => "peer-healthy",
            Self::Degraded => "peer-degraded",
            Self::Disconnected => "peer-disconnected",
        }
    }
}

/// A connected federation peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPeer {
    pub id: String,
    pub name: String,
    pub address: String,
    pub health: PeerHealth,
    pub shared_skills: u32,
    pub latency_ms: Option<u64>,
    pub last_sync: Option<String>,
}

/// Sync status of the federation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Synced,
    Syncing,
    OutOfSync,
    Offline,
}

impl SyncStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Synced => "Synced",
            Self::Syncing => "Syncing...",
            Self::OutOfSync => "Out of sync",
            Self::Offline => "Offline",
        }
    }
}

/// The federation panel view model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPanel {
    pub peers: Vec<FederationPeer>,
    pub sync_status: SyncStatus,
    pub total_shared_skills: u32,
    pub selected_peer_id: Option<String>,
}

impl FederationPanel {
    /// Create an empty federation panel.
    pub fn new() -> Self {
        Self {
            peers: Vec::new(),
            sync_status: SyncStatus::Offline,
            total_shared_skills: 0,
            selected_peer_id: None,
        }
    }

    /// Add a peer.
    pub fn add_peer(&mut self, peer: FederationPeer) {
        self.total_shared_skills += peer.shared_skills;
        self.peers.push(peer);
    }

    /// Remove a peer.
    pub fn remove_peer(&mut self, id: &str) {
        if let Some(idx) = self.peers.iter().position(|p| p.id == id) {
            self.total_shared_skills -= self.peers[idx].shared_skills;
            self.peers.remove(idx);
        }
        if self.selected_peer_id.as_deref() == Some(id) {
            self.selected_peer_id = None;
        }
    }

    /// Count connected peers.
    pub fn connected_count(&self) -> usize {
        self.peers.iter().filter(|p| p.health != PeerHealth::Disconnected).count()
    }

    /// Get the selected peer.
    pub fn selected_peer(&self) -> Option<&FederationPeer> {
        self.selected_peer_id
            .as_ref()
            .and_then(|id| self.peers.iter().find(|p| p.id == *id))
    }

    /// Update peer health.
    pub fn set_peer_health(&mut self, id: &str, health: PeerHealth) {
        if let Some(peer) = self.peers.iter_mut().find(|p| p.id == id) {
            peer.health = health;
        }
    }
}

impl Default for FederationPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_peer(id: &str, name: &str) -> FederationPeer {
        FederationPeer {
            id: id.into(),
            name: name.into(),
            address: format!("127.0.0.1:{}", id),
            health: PeerHealth::Healthy,
            shared_skills: 5,
            latency_ms: Some(12),
            last_sync: None,
        }
    }

    #[test]
    fn test_federation_panel_creation() {
        let panel = FederationPanel::new();
        assert!(panel.peers.is_empty());
        assert_eq!(panel.sync_status, SyncStatus::Offline);
    }

    #[test]
    fn test_add_peer() {
        let mut panel = FederationPanel::new();
        panel.add_peer(sample_peer("p1", "Peer 1"));
        assert_eq!(panel.peers.len(), 1);
        assert_eq!(panel.total_shared_skills, 5);
        assert_eq!(panel.connected_count(), 1);
    }

    #[test]
    fn test_remove_peer() {
        let mut panel = FederationPanel::new();
        panel.add_peer(sample_peer("p1", "Peer 1"));
        panel.add_peer(sample_peer("p2", "Peer 2"));
        panel.remove_peer("p1");
        assert_eq!(panel.peers.len(), 1);
        assert_eq!(panel.total_shared_skills, 5);
    }

    #[test]
    fn test_connected_count() {
        let mut panel = FederationPanel::new();
        panel.add_peer(sample_peer("p1", "Peer 1"));
        let mut p2 = sample_peer("p2", "Peer 2");
        p2.health = PeerHealth::Disconnected;
        panel.add_peer(p2);
        assert_eq!(panel.connected_count(), 1);
    }

    #[test]
    fn test_set_peer_health() {
        let mut panel = FederationPanel::new();
        panel.add_peer(sample_peer("p1", "Peer 1"));
        panel.set_peer_health("p1", PeerHealth::Degraded);
        assert_eq!(panel.peers[0].health, PeerHealth::Degraded);
    }

    #[test]
    fn test_peer_health_css() {
        assert_eq!(PeerHealth::Healthy.css_class(), "peer-healthy");
        assert_eq!(PeerHealth::Degraded.css_class(), "peer-degraded");
        assert_eq!(PeerHealth::Disconnected.css_class(), "peer-disconnected");
    }
}
