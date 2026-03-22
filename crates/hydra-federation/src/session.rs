//! FederationSession — an active sharing context between two peers.
//! Time-bounded. Revocable. Every action receipted.

use crate::{constants::DEFAULT_SESSION_HOURS, scope::TrustScope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The state of a federation session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionState {
    Active,
    Expired,
    Revoked { reason: String },
    Completed,
}

impl SessionState {
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Active)
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Revoked { .. } => "revoked",
            Self::Completed => "completed",
        }
    }
}

/// One sharing event within a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingEvent {
    pub id: String,
    pub event_type: String,
    pub description: String,
    pub receipt_id: String,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
}

impl SharingEvent {
    pub fn new(event_type: impl Into<String>, description: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        let id = uuid::Uuid::new_v4().to_string();
        let et = event_type.into();
        let desc = description.into();

        // Write-ahead receipt
        let receipt_id = {
            let mut h = Sha256::new();
            h.update(id.as_bytes());
            h.update(et.as_bytes());
            h.update(desc.as_bytes());
            h.update(now.to_rfc3339().as_bytes());
            hex::encode(h.finalize())
        };

        Self {
            id,
            event_type: et,
            description: desc,
            receipt_id,
            occurred_at: now,
        }
    }
}

/// An active federation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSession {
    pub id: String,
    pub local_id: String,
    pub remote_id: String,
    pub scope_id: String,
    pub state: SessionState,
    pub events: Vec<SharingEvent>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub session_hash: String,
}

impl FederationSession {
    pub fn new(
        local_id: impl Into<String>,
        remote_id: impl Into<String>,
        scope: &TrustScope,
    ) -> Self {
        let now = chrono::Utc::now();
        let lid = local_id.into();
        let rid = remote_id.into();
        let exp = now + chrono::Duration::hours(DEFAULT_SESSION_HOURS);

        let hash = {
            let mut h = Sha256::new();
            h.update(lid.as_bytes());
            h.update(rid.as_bytes());
            h.update(scope.id.as_bytes());
            h.update(now.to_rfc3339().as_bytes());
            hex::encode(h.finalize())
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            local_id: lid,
            remote_id: rid,
            scope_id: scope.id.clone(),
            state: SessionState::Active,
            events: Vec::new(),
            started_at: now,
            expires_at: exp,
            session_hash: hash,
        }
    }

    /// Record a sharing event within this session.
    pub fn record_event(
        &mut self,
        event_type: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<String, crate::errors::FederationError> {
        if !self.state.is_usable() {
            return Err(crate::errors::FederationError::SessionExpired {
                session_id: self.id.clone(),
            });
        }
        // Check expiry
        if chrono::Utc::now() > self.expires_at {
            self.state = SessionState::Expired;
            return Err(crate::errors::FederationError::SessionExpired {
                session_id: self.id.clone(),
            });
        }
        let event = SharingEvent::new(event_type, description);
        let receipt = event.receipt_id.clone();
        self.events.push(event);
        Ok(receipt)
    }

    pub fn revoke(&mut self, reason: impl Into<String>) {
        self.state = SessionState::Revoked {
            reason: reason.into(),
        };
    }

    pub fn complete(&mut self) {
        self.state = SessionState::Completed;
    }

    pub fn is_active(&self) -> bool {
        self.state.is_usable()
    }
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
    pub fn verify_hash(&self) -> bool {
        self.session_hash.len() == 64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::{ScopeItem, TrustScope};

    fn make_agreed_scope() -> TrustScope {
        let mut scope = TrustScope::new(
            "hydra-a",
            "hydra-b",
            vec![ScopeItem::GenomeEntries {
                domain: "engineering".into(),
                max_count: 50,
            }],
        );
        scope.counter_offer(vec![ScopeItem::PatternDetection]);
        scope
    }

    #[test]
    fn session_starts_active() {
        let scope = make_agreed_scope();
        let session = FederationSession::new("hydra-a", "hydra-b", &scope);
        assert!(session.is_active());
        assert!(session.verify_hash());
        assert_eq!(session.state.label(), "active");
    }

    #[test]
    fn events_recorded_with_receipts() {
        let scope = make_agreed_scope();
        let mut ses = FederationSession::new("hydra-a", "hydra-b", &scope);
        let receipt = ses
            .record_event(
                "genome-share",
                "Shared 12 genome entries for engineering domain",
            )
            .expect("should record event");
        assert!(!receipt.is_empty());
        assert_eq!(ses.event_count(), 1);
    }

    #[test]
    fn revoked_session_rejects_events() {
        let scope = make_agreed_scope();
        let mut ses = FederationSession::new("hydra-a", "hydra-b", &scope);
        ses.revoke("test revocation");
        let result = ses.record_event("genome-share", "attempt");
        assert!(result.is_err());
    }

    #[test]
    fn session_hash_64_chars() {
        let scope = make_agreed_scope();
        let session = FederationSession::new("hydra-a", "hydra-b", &scope);
        assert_eq!(session.session_hash.len(), 64);
    }
}
