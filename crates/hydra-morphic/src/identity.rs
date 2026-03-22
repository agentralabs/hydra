//! Morphic identity — the continuous, evolving identity of a Hydra instance.

use serde::{Deserialize, Serialize};

use crate::constants::{IDENTITY_DISTANCE_THRESHOLD, MORPHIC_HISTORY_MAX};
use crate::errors::MorphicError;
use crate::event::{MorphicEvent, MorphicEventKind};
use crate::signature::MorphicSignature;
use hydra_constitution::{ConstitutionChecker, LawCheckContext};

/// A Hydra instance's morphic identity, tracking its evolution over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphicIdentity {
    /// The morphic signature — unforgeable hash chain.
    pub signature: MorphicSignature,
    /// History of morphic events.
    pub history: Vec<MorphicEvent>,
}

impl MorphicIdentity {
    /// Create a genesis identity for a new Hydra instance.
    pub fn genesis() -> Self {
        Self {
            signature: MorphicSignature::genesis(),
            history: Vec::new(),
        }
    }

    /// Record a morphic event and deepen the signature.
    /// Constitutional Law 2 (Identity Integrity) is enforced here.
    pub fn record_event(&mut self, kind: MorphicEventKind) -> Result<(), MorphicError> {
        // Constitutional check: Law 2 (Identity Integrity)
        let checker = ConstitutionChecker::new();
        let event_desc = format!("{kind:?}");
        let ctx = LawCheckContext::new(
            &self.signature.current_hash,
            "identity.deepen",
        )
        .with_meta("event_kind", &event_desc)
        .with_meta("depth", self.history.len().to_string());
        if let Err(e) = checker.check_strict(&ctx) {
            eprintln!("hydra: identity deepening BLOCKED by constitution: {e}");
            return Err(MorphicError::ConstitutionalViolation {
                reason: format!("{e}"),
            });
        }

        if self.history.len() >= MORPHIC_HISTORY_MAX {
            return Err(MorphicError::HistoryFull {
                count: self.history.len(),
                max: MORPHIC_HISTORY_MAX,
            });
        }

        let prior_hash = self.signature.current_hash.clone();
        self.signature.deepen(&event_desc);

        let event = MorphicEvent::new(kind, prior_hash);
        self.history.push(event);
        Ok(())
    }

    /// Record a system restart in the identity.
    pub fn record_restart(&mut self) -> Result<(), MorphicError> {
        self.record_event(MorphicEventKind::SystemRestart {
            reason: "scheduled".to_string(),
        })?;
        // Note: signature.record_restart already called via deepen in record_event,
        // but we need to track restart_count separately
        self.signature.restart_count += 1;
        Ok(())
    }

    /// Determine if another identity is the same entity (within distance threshold).
    pub fn is_same_entity(&self, other: &MorphicIdentity) -> bool {
        self.signature.distance(&other.signature) <= IDENTITY_DISTANCE_THRESHOLD
    }

    /// Return the current depth of the morphic signature.
    pub fn depth(&self) -> u64 {
        self.signature.depth
    }

    /// Return a human-readable summary of this identity.
    pub fn summary(&self) -> String {
        format!(
            "MorphicIdentity: depth={}, events={}, restarts={}, hash={}...",
            self.signature.depth,
            self.history.len(),
            self.signature.restart_count,
            &self.signature.current_hash[..16.min(self.signature.current_hash.len())],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_identity_has_zero_depth() {
        let id = MorphicIdentity::genesis();
        assert_eq!(id.depth(), 0);
        assert!(id.history.is_empty());
    }

    #[test]
    fn record_event_deepens_signature() {
        let mut id = MorphicIdentity::genesis();
        id.record_event(MorphicEventKind::CapabilityAdded {
            name: "test".into(),
        })
        .expect("record");
        assert_eq!(id.depth(), 1);
        assert_eq!(id.history.len(), 1);
    }

    #[test]
    fn same_entity_at_genesis() {
        let a = MorphicIdentity::genesis();
        // Clone immediately — still same entity
        let b = a.clone();
        assert!(a.is_same_entity(&b));
    }

    #[test]
    fn summary_contains_depth() {
        let id = MorphicIdentity::genesis();
        assert!(id.summary().contains("depth=0"));
    }
}
