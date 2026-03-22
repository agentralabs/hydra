//! FederationPeer — another Hydra instance.
//! Each peer has a stable identity and a known capability set.

use serde::{Deserialize, Serialize};

/// What a peer has told us it can offer in federation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerCapability {
    /// Will share genome entries for these domains.
    GenomeSharing { domains: Vec<String> },
    /// Will provide wisdom judgments.
    WisdomSharing { domains: Vec<String> },
    /// Will share crystallized artifacts.
    ArtifactSharing { kinds: Vec<String> },
    /// Will participate in distributed pattern detection.
    PatternCollective,
    /// Will provide settlement execution.
    SettlementExecution,
}

impl PeerCapability {
    pub fn label(&self) -> String {
        match self {
            Self::GenomeSharing { domains } => format!("genome:{}", domains.join(",")),
            Self::WisdomSharing { domains } => format!("wisdom:{}", domains.join(",")),
            Self::ArtifactSharing { kinds } => format!("artifacts:{}", kinds.join(",")),
            Self::PatternCollective => "pattern-collective".into(),
            Self::SettlementExecution => "settlement".into(),
        }
    }
}

/// The contact information for a peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAddress {
    /// Primary address (e.g. "hydra-b.agentra.io:7474")
    pub primary: String,
    /// Fallback addresses.
    pub fallbacks: Vec<String>,
    /// Protocol hint.
    pub protocol: String,
}

impl PeerAddress {
    pub fn new(primary: impl Into<String>) -> Self {
        Self {
            primary: primary.into(),
            fallbacks: Vec::new(),
            protocol: "hydra-federation/1.0".into(),
        }
    }
}

/// One known peer Hydra instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPeer {
    pub peer_id: String,
    pub display_name: String,
    pub address: PeerAddress,
    pub capabilities: Vec<PeerCapability>,
    /// SHA256 fingerprint of the peer's public key.
    pub key_fingerprint: String,
    pub trust_score: f64,
    pub last_contact: Option<chrono::DateTime<chrono::Utc>>,
    pub is_verified: bool,
    pub registered_at: chrono::DateTime<chrono::Utc>,
}

impl FederationPeer {
    pub fn new(
        peer_id: impl Into<String>,
        display_name: impl Into<String>,
        address: PeerAddress,
        capabilities: Vec<PeerCapability>,
        key_fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            peer_id: peer_id.into(),
            display_name: display_name.into(),
            address,
            capabilities,
            key_fingerprint: key_fingerprint.into(),
            trust_score: 0.0, // starts at zero — earns trust
            last_contact: None,
            is_verified: false,
            registered_at: chrono::Utc::now(),
        }
    }

    /// Verify the peer's identity using its key fingerprint.
    /// In production: cryptographic challenge-response.
    /// Here: simulate based on fingerprint format.
    pub fn verify_identity(&mut self) -> bool {
        // A valid fingerprint is 64 hex chars (SHA256)
        let valid = self.key_fingerprint.len() == 64
            && self.key_fingerprint.chars().all(|c| c.is_ascii_hexdigit());
        self.is_verified = valid;
        valid
    }

    pub fn update_trust(&mut self, score: f64) {
        self.trust_score = score.clamp(0.0, 1.0);
        self.last_contact = Some(chrono::Utc::now());
    }

    pub fn meets_federation_threshold(&self) -> bool {
        self.is_verified && self.trust_score >= crate::constants::MIN_FEDERATION_TRUST
    }

    pub fn has_capability(&self, label: &str) -> bool {
        self.capabilities.iter().any(|c| c.label().contains(label))
    }
}

/// Generate a synthetic but valid-looking key fingerprint for tests.
pub fn test_fingerprint(seed: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(seed.as_bytes());
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer(id: &str) -> FederationPeer {
        FederationPeer::new(
            id,
            "Test Peer",
            PeerAddress::new("hydra-test.agentra.io:7474"),
            vec![PeerCapability::GenomeSharing {
                domains: vec!["engineering".into()],
            }],
            test_fingerprint(id),
        )
    }

    #[test]
    fn valid_fingerprint_verifies() {
        let mut peer = make_peer("peer-a");
        assert!(peer.verify_identity());
        assert!(peer.is_verified);
    }

    #[test]
    fn invalid_fingerprint_fails() {
        let mut peer = FederationPeer::new(
            "peer-bad",
            "Bad Peer",
            PeerAddress::new("bad.host:7474"),
            vec![],
            "not-a-valid-fingerprint",
        );
        assert!(!peer.verify_identity());
        assert!(!peer.is_verified);
    }

    #[test]
    fn trust_zero_until_updated() {
        let peer = make_peer("peer-new");
        assert_eq!(peer.trust_score, 0.0);
        assert!(!peer.meets_federation_threshold());
    }

    #[test]
    fn meets_threshold_when_verified_and_trusted() {
        let mut peer = make_peer("peer-good");
        peer.verify_identity();
        peer.update_trust(0.80);
        assert!(peer.meets_federation_threshold());
    }

    #[test]
    fn capability_lookup() {
        let peer = make_peer("peer-a");
        assert!(peer.has_capability("genome"));
        assert!(!peer.has_capability("settlement"));
    }
}
