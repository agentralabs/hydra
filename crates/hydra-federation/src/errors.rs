use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum FederationError {
    #[error("Peer '{peer_id}' identity could not be verified")]
    IdentityVerificationFailed { peer_id: String },

    #[error("Trust negotiation failed: {reason}")]
    NegotiationFailed { reason: String },

    #[error("Trust score {score:.2} below minimum {min:.2} for federation")]
    InsufficientTrust { score: f64, min: f64 },

    #[error("No active session with peer '{peer_id}'")]
    NoActiveSession { peer_id: String },

    #[error("Scope item '{item}' not permitted under agreed trust scope")]
    ScopeViolation { item: String },

    #[error("Peer registry at capacity ({max})")]
    RegistryFull { max: usize },

    #[error("Session '{session_id}' has expired")]
    SessionExpired { session_id: String },
}

impl FederationError {
    pub fn is_hard_stop(&self) -> bool {
        matches!(
            self,
            Self::IdentityVerificationFailed { .. } | Self::ScopeViolation { .. }
        )
    }
}
