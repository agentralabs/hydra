//! FederationEngine — the peer federation coordinator.
//! Layer 6 begins here.

use crate::{
    constants::MIN_FEDERATION_TRUST,
    errors::FederationError,
    peer::FederationPeer,
    registry::PeerRegistry,
    scope::{ScopeItem, TrustScope},
    session::FederationSession,
};

/// The result of a federation handshake.
#[derive(Debug)]
pub struct HandshakeResult {
    pub peer_id: String,
    pub session_id: String,
    pub scope_id: String,
    pub trust_score: f64,
    pub our_offers: usize,
    pub their_offers: usize,
}

/// The federation engine.
pub struct FederationEngine {
    pub local_peer_id: String,
    pub registry: PeerRegistry,
}

impl FederationEngine {
    pub fn new(local_peer_id: impl Into<String>) -> Self {
        Self {
            local_peer_id: local_peer_id.into(),
            registry: PeerRegistry::new(),
        }
    }

    /// Register a newly discovered peer.
    pub fn register_peer(&mut self, peer: FederationPeer) -> Result<(), FederationError> {
        self.registry.register_peer(peer)
    }

    /// Verify a peer's identity.
    pub fn verify_peer(&mut self, peer_id: &str) -> Result<(), FederationError> {
        let peer = self.registry.get_peer_mut(peer_id).ok_or_else(|| {
            FederationError::IdentityVerificationFailed {
                peer_id: peer_id.to_string(),
            }
        })?;

        if !peer.verify_identity() {
            return Err(FederationError::IdentityVerificationFailed {
                peer_id: peer_id.to_string(),
            });
        }
        Ok(())
    }

    /// Set trust score for a peer (from observed interactions).
    pub fn update_peer_trust(&mut self, peer_id: &str, score: f64) {
        if let Some(peer) = self.registry.get_peer_mut(peer_id) {
            peer.update_trust(score);
        }
    }

    /// Initiate trust scope negotiation with a peer.
    pub fn propose_scope(
        &mut self,
        peer_id: &str,
        our_offers: Vec<ScopeItem>,
    ) -> Result<String, FederationError> {
        let peer =
            self.registry
                .get_peer(peer_id)
                .ok_or_else(|| FederationError::NoActiveSession {
                    peer_id: peer_id.to_string(),
                })?;

        if !peer.meets_federation_threshold() {
            return Err(FederationError::InsufficientTrust {
                score: peer.trust_score,
                min: MIN_FEDERATION_TRUST,
            });
        }

        let scope = TrustScope::new(&self.local_peer_id, peer_id, our_offers);
        let scope_id = scope.id.clone();
        self.registry.store_scope(scope);
        Ok(scope_id)
    }

    /// Accept a peer's counter-offer and finalize the scope.
    pub fn accept_counter_offer(
        &mut self,
        peer_id: &str,
        their_offers: Vec<ScopeItem>,
    ) -> Result<(), FederationError> {
        let scope = self.registry.get_scope_mut(peer_id).ok_or_else(|| {
            FederationError::NegotiationFailed {
                reason: format!("No pending scope for peer '{}'", peer_id),
            }
        })?;

        scope.counter_offer(their_offers);

        if !scope.is_active() {
            return Err(FederationError::NegotiationFailed {
                reason: "Counter-offer did not result in agreement".into(),
            });
        }
        Ok(())
    }

    /// Establish a session after scope is agreed.
    pub fn establish_session(&mut self, peer_id: &str) -> Result<String, FederationError> {
        let scope =
            self.registry
                .get_scope(peer_id)
                .ok_or_else(|| FederationError::NoActiveSession {
                    peer_id: peer_id.to_string(),
                })?;

        if !scope.is_active() {
            return Err(FederationError::NegotiationFailed {
                reason: "Scope is not in agreed state".into(),
            });
        }

        let session = FederationSession::new(&self.local_peer_id, peer_id, scope);
        let session_id = session.id.clone();
        self.registry.store_session(session);
        Ok(session_id)
    }

    /// Full handshake: verify -> trust check -> negotiate scope -> establish session.
    pub fn handshake(
        &mut self,
        peer_id: &str,
        our_offers: Vec<ScopeItem>,
        their_offers: Vec<ScopeItem>,
        trust_score: f64,
    ) -> Result<HandshakeResult, FederationError> {
        // Step 1: Verify identity
        self.verify_peer(peer_id)?;

        // Step 2: Set trust score
        self.update_peer_trust(peer_id, trust_score);

        // Step 3: Propose scope
        self.propose_scope(peer_id, our_offers.clone())?;

        // Step 4: Accept counter-offer
        self.accept_counter_offer(peer_id, their_offers.clone())?;

        // Step 5: Establish session
        let session_id = self.establish_session(peer_id)?;

        let scope_id = self
            .registry
            .get_scope(peer_id)
            .map(|s| s.id.clone())
            .unwrap_or_default();

        Ok(HandshakeResult {
            peer_id: peer_id.to_string(),
            session_id,
            scope_id,
            trust_score,
            our_offers: our_offers.len(),
            their_offers: their_offers.len(),
        })
    }

    /// Record a sharing event in an active session.
    pub fn record_sharing(
        &mut self,
        session_id: &str,
        event_type: &str,
        description: &str,
    ) -> Result<String, FederationError> {
        // Scope check first
        let scope_ok = self
            .registry
            .active_sessions()
            .iter()
            .any(|s| s.id == session_id);

        if !scope_ok {
            return Err(FederationError::NoActiveSession {
                peer_id: session_id.to_string(),
            });
        }

        let session = self.registry.get_session_mut(session_id).ok_or_else(|| {
            FederationError::NoActiveSession {
                peer_id: session_id.to_string(),
            }
        })?;

        session.record_event(event_type, description)
    }

    /// Revoke a session.
    pub fn revoke_session(&mut self, session_id: &str, reason: &str) {
        if let Some(session) = self.registry.get_session_mut(session_id) {
            session.revoke(reason);
        }
    }

    pub fn peer_count(&self) -> usize {
        self.registry.peer_count()
    }
    pub fn active_session_count(&self) -> usize {
        self.registry.active_session_count()
    }
    pub fn active_scope_count(&self) -> usize {
        self.registry.active_scope_count()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "federation: peers={} trusted={} sessions={} scopes={}",
            self.peer_count(),
            self.registry.trusted_peers().len(),
            self.active_session_count(),
            self.active_scope_count(),
        )
    }
}

impl Default for FederationEngine {
    fn default() -> Self {
        Self::new("hydra-local")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::{test_fingerprint, PeerAddress, PeerCapability};

    fn make_engine() -> FederationEngine {
        FederationEngine::new("hydra-a")
    }

    fn register_peer(engine: &mut FederationEngine, id: &str) {
        let peer = FederationPeer::new(
            id,
            "Test Peer",
            PeerAddress::new(format!("{}.host:7474", id)),
            vec![PeerCapability::GenomeSharing {
                domains: vec!["engineering".into()],
            }],
            test_fingerprint(id),
        );
        engine.register_peer(peer).expect("should register peer");
    }

    #[test]
    fn full_handshake_succeeds() {
        let mut engine = make_engine();
        register_peer(&mut engine, "hydra-b");

        let result = engine
            .handshake(
                "hydra-b",
                vec![ScopeItem::GenomeEntries {
                    domain: "engineering".into(),
                    max_count: 50,
                }],
                vec![ScopeItem::GenomeEntries {
                    domain: "fintech".into(),
                    max_count: 30,
                }],
                0.80,
            )
            .expect("handshake should succeed");

        assert_eq!(result.peer_id, "hydra-b");
        assert!(result.trust_score >= MIN_FEDERATION_TRUST);
        assert_eq!(engine.active_session_count(), 1);
        assert_eq!(engine.active_scope_count(), 1);
    }

    #[test]
    fn low_trust_handshake_fails() {
        let mut engine = make_engine();
        register_peer(&mut engine, "hydra-low");

        let result = engine.handshake(
            "hydra-low",
            vec![ScopeItem::PatternDetection],
            vec![ScopeItem::PatternDetection],
            0.40, // below MIN_FEDERATION_TRUST
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FederationError::InsufficientTrust { .. }
        ));
    }

    #[test]
    fn session_records_events() {
        let mut engine = make_engine();
        register_peer(&mut engine, "hydra-c");

        let result = engine
            .handshake(
                "hydra-c",
                vec![ScopeItem::GenomeEntries {
                    domain: "engineering".into(),
                    max_count: 20,
                }],
                vec![ScopeItem::PatternDetection],
                0.75,
            )
            .expect("handshake should succeed");

        let receipt = engine
            .record_sharing(
                &result.session_id,
                "genome-share",
                "Shared 8 genome entries for engineering domain. Provenance: hydra-a.",
            )
            .expect("should record sharing");
        assert!(!receipt.is_empty());
    }

    #[test]
    fn revoked_session_count_drops() {
        let mut engine = make_engine();
        register_peer(&mut engine, "hydra-d");
        let result = engine
            .handshake(
                "hydra-d",
                vec![ScopeItem::PatternDetection],
                vec![ScopeItem::PatternDetection],
                0.78,
            )
            .expect("handshake should succeed");
        assert_eq!(engine.active_session_count(), 1);
        engine.revoke_session(&result.session_id, "test revocation");
        assert_eq!(engine.active_session_count(), 0);
    }

    #[test]
    fn summary_format() {
        let engine = make_engine();
        let s = engine.summary();
        assert!(s.contains("federation:"));
        assert!(s.contains("peers="));
        assert!(s.contains("sessions="));
    }
}
