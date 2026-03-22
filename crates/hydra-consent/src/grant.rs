//! ConsentGrant — one specific consent permission.
//! Specific. Versioned. Revocable. Time-bounded.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// What is permitted under this grant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConsentScope {
    /// Share genome entries for a domain.
    GenomeSharing { domain: String, max_entries: usize },
    /// Share wisdom judgments.
    WisdomSharing { domain: String },
    /// Share crystallized artifacts of a kind.
    ArtifactSharing { kind: String },
    /// Participate in pattern collective.
    PatternParticipation,
    /// Receive settlement capability.
    SettlementAccess,
}

impl ConsentScope {
    pub fn label(&self) -> String {
        match self {
            Self::GenomeSharing { domain, .. } => format!("genome:{domain}"),
            Self::WisdomSharing { domain } => format!("wisdom:{domain}"),
            Self::ArtifactSharing { kind } => format!("artifact:{kind}"),
            Self::PatternParticipation => "pattern".into(),
            Self::SettlementAccess => "settlement".into(),
        }
    }

    pub fn covers(&self, action: &str) -> bool {
        action.contains(&self.label())
    }
}

/// The state of a consent grant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GrantState {
    Active,
    Expired,
    Revoked { reason: String },
    Exhausted,
}

impl GrantState {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Active)
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Revoked { .. } => "revoked",
            Self::Exhausted => "exhausted",
        }
    }
}

/// One consent grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentGrant {
    pub id: String,
    pub peer_id: String,
    pub scope: ConsentScope,
    pub state: GrantState,
    pub version: u32,
    pub max_uses: Option<usize>,
    pub use_count: usize,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub integrity_hash: String,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ConsentGrant {
    pub fn new(
        peer_id: impl Into<String>,
        scope: ConsentScope,
        max_uses: Option<usize>,
        valid_days: i64,
    ) -> Self {
        let now = chrono::Utc::now();
        let pid = peer_id.into();
        let exp = now + chrono::Duration::days(valid_days);
        let scope_label = scope.label();

        let hash = {
            let mut h = Sha256::new();
            h.update(pid.as_bytes());
            h.update(scope_label.as_bytes());
            h.update(now.to_rfc3339().as_bytes());
            h.update(exp.to_rfc3339().as_bytes());
            hex::encode(h.finalize())
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            peer_id: pid,
            scope,
            state: GrantState::Active,
            version: 1,
            max_uses,
            use_count: 0,
            expires_at: exp,
            integrity_hash: hash,
            granted_at: now,
            revoked_at: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.state.is_valid() && chrono::Utc::now() <= self.expires_at
    }

    pub fn covers(&self, action: &str) -> bool {
        self.is_valid() && self.scope.covers(action)
    }

    pub fn record_use(&mut self) -> Result<(), crate::errors::ConsentError> {
        if !self.is_valid() && chrono::Utc::now() > self.expires_at {
            self.state = GrantState::Expired;
            return Err(crate::errors::ConsentError::Expired {
                grant_id: self.id.clone(),
            });
        }
        self.use_count += 1;
        if let Some(max) = self.max_uses {
            if self.use_count >= max {
                self.state = GrantState::Exhausted;
            }
        }
        Ok(())
    }

    pub fn revoke(&mut self, reason: impl Into<String>) {
        self.state = GrantState::Revoked {
            reason: reason.into(),
        };
        self.revoked_at = Some(chrono::Utc::now());
    }

    pub fn verify_integrity(&self) -> bool {
        !self.integrity_hash.is_empty() && self.integrity_hash.len() == 64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_starts_active() {
        let g = ConsentGrant::new(
            "peer-b",
            ConsentScope::GenomeSharing {
                domain: "engineering".into(),
                max_entries: 50,
            },
            Some(100),
            30,
        );
        assert!(g.is_valid());
        assert_eq!(g.state.label(), "active");
        assert!(g.verify_integrity());
    }

    #[test]
    fn grant_covers_action() {
        let g = ConsentGrant::new(
            "peer-b",
            ConsentScope::GenomeSharing {
                domain: "engineering".into(),
                max_entries: 50,
            },
            None,
            30,
        );
        assert!(g.covers("genome:engineering"));
        assert!(!g.covers("genome:fintech"));
    }

    #[test]
    fn revoked_grant_not_valid() {
        let mut g = ConsentGrant::new("peer-b", ConsentScope::PatternParticipation, None, 30);
        g.revoke("no longer trusted");
        assert!(!g.is_valid());
    }

    #[test]
    fn exhausted_after_max_uses() {
        let mut g = ConsentGrant::new(
            "peer-b",
            ConsentScope::WisdomSharing {
                domain: "fintech".into(),
            },
            Some(3),
            30,
        );
        g.record_use().expect("use 1");
        g.record_use().expect("use 2");
        g.record_use().expect("use 3");
        assert_eq!(g.state.label(), "exhausted");
    }

    #[test]
    fn integrity_hash_64_chars() {
        let g = ConsentGrant::new("peer", ConsentScope::PatternParticipation, None, 30);
        assert_eq!(g.integrity_hash.len(), 64);
    }
}
