//! PeerRegistry — all known peer Hydra instances.

use crate::{
    constants::MAX_REGISTERED_PEERS, errors::FederationError, peer::FederationPeer,
    scope::TrustScope, session::FederationSession,
};
use std::collections::HashMap;

/// The peer registry.
#[derive(Debug, Default)]
pub struct PeerRegistry {
    peers: HashMap<String, FederationPeer>,
    scopes: HashMap<String, TrustScope>,          // key: peer_id
    sessions: HashMap<String, FederationSession>, // key: session_id
}

impl PeerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // -- PEERS --

    pub fn register_peer(&mut self, peer: FederationPeer) -> Result<(), FederationError> {
        if self.peers.len() >= MAX_REGISTERED_PEERS {
            return Err(FederationError::RegistryFull {
                max: MAX_REGISTERED_PEERS,
            });
        }
        self.peers.insert(peer.peer_id.clone(), peer);
        Ok(())
    }

    pub fn get_peer(&self, peer_id: &str) -> Option<&FederationPeer> {
        self.peers.get(peer_id)
    }

    pub fn get_peer_mut(&mut self, peer_id: &str) -> Option<&mut FederationPeer> {
        self.peers.get_mut(peer_id)
    }

    pub fn verified_peers(&self) -> Vec<&FederationPeer> {
        self.peers.values().filter(|p| p.is_verified).collect()
    }

    pub fn trusted_peers(&self) -> Vec<&FederationPeer> {
        self.peers
            .values()
            .filter(|p| p.meets_federation_threshold())
            .collect()
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    // -- SCOPES --

    pub fn store_scope(&mut self, scope: TrustScope) {
        self.scopes.insert(scope.remote_peer_id.clone(), scope);
    }

    pub fn get_scope(&self, peer_id: &str) -> Option<&TrustScope> {
        self.scopes.get(peer_id)
    }

    pub fn get_scope_mut(&mut self, peer_id: &str) -> Option<&mut TrustScope> {
        self.scopes.get_mut(peer_id)
    }

    pub fn active_scope_count(&self) -> usize {
        self.scopes.values().filter(|s| s.is_active()).count()
    }

    // -- SESSIONS --

    pub fn store_session(&mut self, session: FederationSession) {
        self.sessions.insert(session.id.clone(), session);
    }

    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut FederationSession> {
        self.sessions.get_mut(session_id)
    }

    pub fn active_sessions(&self) -> Vec<&FederationSession> {
        self.sessions.values().filter(|s| s.is_active()).collect()
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
    pub fn active_session_count(&self) -> usize {
        self.active_sessions().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::{test_fingerprint, FederationPeer, PeerAddress, PeerCapability};

    fn make_peer(id: &str) -> FederationPeer {
        let mut p = FederationPeer::new(
            id,
            "Test",
            PeerAddress::new("host:7474"),
            vec![PeerCapability::PatternCollective],
            test_fingerprint(id),
        );
        p.verify_identity();
        p.update_trust(0.80);
        p
    }

    #[test]
    fn register_and_retrieve() {
        let mut reg = PeerRegistry::new();
        reg.register_peer(make_peer("peer-a"))
            .expect("should register");
        assert_eq!(reg.peer_count(), 1);
        assert!(reg.get_peer("peer-a").is_some());
    }

    #[test]
    fn trusted_peers_filtered() {
        let mut reg = PeerRegistry::new();
        let mut low = make_peer("low-trust");
        low.update_trust(0.40);
        reg.register_peer(low).expect("should register");
        reg.register_peer(make_peer("high-trust"))
            .expect("should register");
        assert_eq!(reg.trusted_peers().len(), 1);
    }
}
