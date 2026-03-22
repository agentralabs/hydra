//! ConsentRegistry — all active consent grants.

use crate::grant::ConsentGrant;
use std::collections::HashMap;

/// The consent registry.
#[derive(Debug, Default)]
pub struct ConsentRegistry {
    /// peer_id -> list of grants
    grants: HashMap<String, Vec<ConsentGrant>>,
}

impl ConsentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn grant(&mut self, grant: ConsentGrant) {
        self.grants
            .entry(grant.peer_id.clone())
            .or_default()
            .push(grant);
    }

    /// Find the first valid grant for a peer + action.
    pub fn find_grant(&self, peer_id: &str, action: &str) -> Option<&ConsentGrant> {
        self.grants.get(peer_id)?.iter().find(|g| g.covers(action))
    }

    pub fn find_grant_mut(&mut self, peer_id: &str, action: &str) -> Option<&mut ConsentGrant> {
        self.grants
            .get_mut(peer_id)?
            .iter_mut()
            .find(|g| g.covers(action))
    }

    pub fn revoke_all_for_peer(&mut self, peer_id: &str, reason: &str) {
        if let Some(grants) = self.grants.get_mut(peer_id) {
            for g in grants.iter_mut() {
                if g.is_valid() {
                    g.revoke(reason);
                }
            }
        }
    }

    pub fn active_grant_count(&self) -> usize {
        self.grants
            .values()
            .flat_map(|gs| gs.iter())
            .filter(|g| g.is_valid())
            .count()
    }

    pub fn total_grant_count(&self) -> usize {
        self.grants.values().map(|gs| gs.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grant::{ConsentGrant, ConsentScope};

    #[test]
    fn grant_and_find() {
        let mut reg = ConsentRegistry::new();
        reg.grant(ConsentGrant::new(
            "peer-b",
            ConsentScope::GenomeSharing {
                domain: "engineering".into(),
                max_entries: 50,
            },
            None,
            30,
        ));
        assert!(reg.find_grant("peer-b", "genome:engineering").is_some());
        assert!(reg.find_grant("peer-b", "wisdom:engineering").is_none());
    }

    #[test]
    fn revoke_all_for_peer() {
        let mut reg = ConsentRegistry::new();
        reg.grant(ConsentGrant::new(
            "peer-b",
            ConsentScope::PatternParticipation,
            None,
            30,
        ));
        assert_eq!(reg.active_grant_count(), 1);
        reg.revoke_all_for_peer("peer-b", "peer no longer trusted");
        assert_eq!(reg.active_grant_count(), 0);
    }
}
