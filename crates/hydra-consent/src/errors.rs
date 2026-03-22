use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ConsentError {
    #[error("No active consent grant for peer '{peer_id}' — action '{action}'")]
    NoConsent { peer_id: String, action: String },

    #[error("Consent grant '{grant_id}' has expired")]
    Expired { grant_id: String },

    #[error("Consent grant '{grant_id}' has been revoked")]
    Revoked { grant_id: String },

    #[error("Action '{action}' exceeds consent grant limit for peer '{peer_id}'")]
    LimitExceeded { peer_id: String, action: String },

    #[error("Consent audit store at capacity ({max})")]
    StoreFull { max: usize },
}

impl ConsentError {
    pub fn is_hard_stop(&self) -> bool {
        matches!(self, Self::Revoked { .. } | Self::NoConsent { .. })
    }
}
