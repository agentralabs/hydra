//! SkillSharing — share compiled skills between peers with permission model.

use std::collections::HashMap;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::peer::{PeerId, PeerInfo};

#[derive(Debug, Error)]
pub enum SharingError {
    #[error("insufficient trust level for sharing")]
    InsufficientTrust,
    #[error("skill not found: {0}")]
    SkillNotFound(String),
    #[error("sharing denied by policy")]
    PolicyDenied,
}

/// A shared skill descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSkill {
    pub id: String,
    pub name: String,
    pub version: String,
    pub signature: String,
    pub owner_peer: PeerId,
    pub share_level: ShareLevel,
}

/// What level of sharing is permitted
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ShareLevel {
    /// Not shared
    Private,
    /// Metadata only (name, description)
    MetadataOnly,
    /// Full skill definition, read-only execution
    ReadOnly,
    /// Full sharing, can modify locally
    Full,
}

/// Sharing policy for outbound skills
#[derive(Debug, Clone)]
pub struct SharingPolicy {
    /// Default share level for new peers
    pub default_level: ShareLevel,
    /// Per-skill overrides
    pub skill_overrides: HashMap<String, ShareLevel>,
    /// Per-peer overrides
    pub peer_overrides: HashMap<PeerId, ShareLevel>,
}

impl Default for SharingPolicy {
    fn default() -> Self {
        Self {
            default_level: ShareLevel::MetadataOnly,
            skill_overrides: HashMap::new(),
            peer_overrides: HashMap::new(),
        }
    }
}

/// Skill sharing manager
pub struct SkillSharing {
    /// Skills available for sharing
    offered: RwLock<HashMap<String, SharedSkill>>,
    /// Skills received from peers
    received: RwLock<HashMap<String, SharedSkill>>,
    /// Sharing policy
    policy: RwLock<SharingPolicy>,
}

impl SkillSharing {
    pub fn new() -> Self {
        Self {
            offered: RwLock::new(HashMap::new()),
            received: RwLock::new(HashMap::new()),
            policy: RwLock::new(SharingPolicy::default()),
        }
    }

    /// Offer a skill for sharing
    pub fn offer(&self, skill: SharedSkill) {
        self.offered.write().insert(skill.id.clone(), skill);
    }

    /// Check if a peer can access a skill
    pub fn check_permission(
        &self,
        skill_id: &str,
        peer: &PeerInfo,
    ) -> Result<ShareLevel, SharingError> {
        if !peer.allows_skill_sharing() {
            return Err(SharingError::InsufficientTrust);
        }

        let policy = self.policy.read();

        // Check peer override
        if let Some(level) = policy.peer_overrides.get(&peer.id) {
            return Ok(*level);
        }

        // Check skill override
        if let Some(level) = policy.skill_overrides.get(skill_id) {
            return Ok(*level);
        }

        Ok(policy.default_level)
    }

    /// Request a skill from this node
    pub fn handle_request(
        &self,
        skill_id: &str,
        peer: &PeerInfo,
    ) -> Result<SharedSkill, SharingError> {
        let level = self.check_permission(skill_id, peer)?;

        if level == ShareLevel::Private {
            return Err(SharingError::PolicyDenied);
        }

        let offered = self.offered.read();
        let skill = offered
            .get(skill_id)
            .cloned()
            .ok_or_else(|| SharingError::SkillNotFound(skill_id.into()))?;

        Ok(skill)
    }

    /// Receive a skill from a peer
    pub fn receive(&self, skill: SharedSkill) {
        self.received.write().insert(skill.id.clone(), skill);
    }

    /// Get all received skills
    pub fn received_skills(&self) -> Vec<SharedSkill> {
        self.received.read().values().cloned().collect()
    }

    /// Get all offered skills
    pub fn offered_skills(&self) -> Vec<SharedSkill> {
        self.offered.read().values().cloned().collect()
    }

    /// Update sharing policy
    pub fn set_policy(&self, policy: SharingPolicy) {
        *self.policy.write() = policy;
    }

    /// Set per-peer share level
    pub fn set_peer_level(&self, peer_id: &str, level: ShareLevel) {
        self.policy
            .write()
            .peer_overrides
            .insert(peer_id.into(), level);
    }
}

impl Default for SkillSharing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::{FederationType, PeerCapabilities, TrustLevel};

    fn make_peer(id: &str, trust: TrustLevel) -> PeerInfo {
        PeerInfo {
            id: id.into(),
            name: id.into(),
            endpoint: format!("{}:9000", id),
            version: "0.1.0".into(),
            capabilities: PeerCapabilities::default(),
            trust_level: trust,
            federation_type: FederationType::Personal,
            last_seen: chrono::Utc::now().to_rfc3339(),
            active_tasks: 0,
        }
    }

    fn make_skill(id: &str) -> SharedSkill {
        SharedSkill {
            id: id.into(),
            name: format!("skill_{}", id),
            version: "1.0.0".into(),
            signature: "test→deploy".into(),
            owner_peer: "local".into(),
            share_level: ShareLevel::Full,
        }
    }

    #[test]
    fn test_skill_sharing_request() {
        let sharing = SkillSharing::new();
        sharing.offer(make_skill("s1"));

        let trusted_peer = make_peer("p1", TrustLevel::Trusted);
        let result = sharing.handle_request("s1", &trusted_peer);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "s1");
    }

    #[test]
    fn test_skill_sharing_permission_denied() {
        let sharing = SkillSharing::new();
        sharing.offer(make_skill("s1"));

        // Unknown peer can't share
        let unknown_peer = make_peer("p1", TrustLevel::Unknown);
        let result = sharing.handle_request("s1", &unknown_peer);
        assert!(result.is_err());
    }

    #[test]
    fn test_skill_sharing_policy_override() {
        let sharing = SkillSharing::new();
        sharing.offer(make_skill("s1"));
        sharing.set_peer_level("p1", ShareLevel::Private);

        let peer = make_peer("p1", TrustLevel::Trusted);
        let result = sharing.handle_request("s1", &peer);
        assert!(matches!(result, Err(SharingError::PolicyDenied)));
    }

    #[test]
    fn test_skill_sharing_receive() {
        let sharing = SkillSharing::new();
        sharing.receive(make_skill("remote-1"));
        sharing.receive(make_skill("remote-2"));

        assert_eq!(sharing.received_skills().len(), 2);
    }
}
