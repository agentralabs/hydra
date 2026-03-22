//! ConsentEngine — the consent management coordinator.

use crate::{
    audit::{ConsentAuditEntry, ConsentAuditLog},
    errors::ConsentError,
    grant::{ConsentGrant, ConsentScope},
    registry::ConsentRegistry,
};

/// The consent engine.
pub struct ConsentEngine {
    pub registry: ConsentRegistry,
    pub audit: ConsentAuditLog,
}

impl ConsentEngine {
    pub fn new() -> Self {
        Self {
            registry: ConsentRegistry::new(),
            audit: ConsentAuditLog::new(),
        }
    }

    /// Grant consent for a peer to perform an action.
    pub fn grant(
        &mut self,
        peer_id: impl Into<String>,
        scope: ConsentScope,
        max_uses: Option<usize>,
        valid_days: i64,
    ) -> String {
        let grant = ConsentGrant::new(peer_id, scope, max_uses, valid_days);
        let id = grant.id.clone();
        self.registry.grant(grant);
        id
    }

    /// Check consent before a sharing action.
    /// Returns the grant ID if permitted.
    pub fn check(&self, peer_id: &str, action: &str) -> Result<String, ConsentError> {
        let grant =
            self.registry
                .find_grant(peer_id, action)
                .ok_or_else(|| ConsentError::NoConsent {
                    peer_id: peer_id.to_string(),
                    action: action.to_string(),
                })?;

        if !grant.is_valid() {
            return Err(ConsentError::Revoked {
                grant_id: grant.id.clone(),
            });
        }

        Ok(grant.id.clone())
    }

    /// Record a sharing event (check consent first).
    pub fn record_sharing(
        &mut self,
        peer_id: &str,
        action: &str,
        description: &str,
        receipt_id: &str,
    ) -> Result<(), ConsentError> {
        let grant = self
            .registry
            .find_grant_mut(peer_id, action)
            .ok_or_else(|| ConsentError::NoConsent {
                peer_id: peer_id.to_string(),
                action: action.to_string(),
            })?;

        let grant_id = grant.id.clone();
        grant.record_use()?;

        let entry = ConsentAuditEntry::new(&grant_id, peer_id, action, description, receipt_id);
        self.audit.record(entry)?;
        Ok(())
    }

    /// Revoke all consent for a peer.
    pub fn revoke_peer(&mut self, peer_id: &str, reason: &str) {
        self.registry.revoke_all_for_peer(peer_id, reason);
    }

    pub fn active_grant_count(&self) -> usize {
        self.registry.active_grant_count()
    }
    pub fn audit_count(&self) -> usize {
        self.audit.count()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "consent: grants={} active={} audit={}",
            self.registry.total_grant_count(),
            self.active_grant_count(),
            self.audit_count(),
        )
    }
}

impl Default for ConsentEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_and_check() {
        let mut engine = ConsentEngine::new();
        engine.grant(
            "peer-b",
            ConsentScope::GenomeSharing {
                domain: "engineering".into(),
                max_entries: 50,
            },
            None,
            30,
        );
        assert!(engine.check("peer-b", "genome:engineering").is_ok());
        assert!(engine.check("peer-b", "wisdom:engineering").is_err());
    }

    #[test]
    fn record_sharing_audited() {
        let mut engine = ConsentEngine::new();
        engine.grant("peer-b", ConsentScope::PatternParticipation, None, 30);
        engine
            .record_sharing("peer-b", "pattern", "shared pattern data", "receipt-123")
            .expect("should record");
        assert_eq!(engine.audit_count(), 1);
    }

    #[test]
    fn no_consent_error() {
        let engine = ConsentEngine::new();
        let r = engine.check("unknown-peer", "genome:engineering");
        assert!(r.is_err());
        assert!(r.expect_err("should be error").is_hard_stop());
    }

    #[test]
    fn revoke_blocks_further_sharing() {
        let mut engine = ConsentEngine::new();
        engine.grant("peer-b", ConsentScope::PatternParticipation, None, 30);
        engine.revoke_peer("peer-b", "no longer trusted");
        let r = engine.check("peer-b", "pattern");
        assert!(r.is_err());
    }

    #[test]
    fn summary_format() {
        let engine = ConsentEngine::new();
        let s = engine.summary();
        assert!(s.contains("consent:"));
        assert!(s.contains("grants="));
    }
}
