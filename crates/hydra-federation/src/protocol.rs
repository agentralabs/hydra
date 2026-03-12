//! Wire protocol — JSON-RPC messages for federation.

use serde::{Deserialize, Serialize};

use crate::peer::{PeerCapabilities, PeerId};
use crate::sync::SyncEntry;

/// Federation protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationMessage {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

/// Federation protocol methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationMethod {
    /// Initial handshake
    Hello(HelloParams),
    /// Discover peers/capabilities
    Discover(DiscoverParams),
    /// Delegate a task
    Delegate(DelegateParams),
    /// Request a skill
    SkillRequest(SkillRequestParams),
    /// Offer a skill
    SkillOffer(SkillOfferParams),
    /// Sync state
    Sync(SyncParams),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloParams {
    pub peer_id: PeerId,
    pub name: String,
    pub version: String,
    pub capabilities: PeerCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverParams {
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateParams {
    pub task_id: String,
    pub description: String,
    pub requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequestParams {
    pub skill_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOfferParams {
    pub skill_id: String,
    pub name: String,
    pub version: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncParams {
    pub since_version: u64,
    pub changes: Vec<SyncEntry>,
}

/// Federation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationResponse {
    pub id: String,
    pub success: bool,
    pub result: serde_json::Value,
    pub error: Option<String>,
}

impl FederationMessage {
    /// Create a hello message
    pub fn hello(peer_id: &str, name: &str, version: &str, caps: PeerCapabilities) -> Self {
        let params = HelloParams {
            peer_id: peer_id.into(),
            name: name.into(),
            version: version.into(),
            capabilities: caps,
        };
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            method: "federation.hello".into(),
            params: serde_json::to_value(params).unwrap(),
        }
    }

    /// Create a sync message
    pub fn sync(since_version: u64, changes: Vec<SyncEntry>) -> Self {
        let params = SyncParams {
            since_version,
            changes,
        };
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            method: "federation.sync".into(),
            params: serde_json::to_value(params).unwrap(),
        }
    }

    /// Create a skill request message
    pub fn skill_request(skill_id: &str) -> Self {
        let params = SkillRequestParams {
            skill_id: skill_id.into(),
        };
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            method: "federation.skill_request".into(),
            params: serde_json::to_value(params).unwrap(),
        }
    }
}

impl FederationResponse {
    pub fn success(id: &str, result: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            success: true,
            result,
            error: None,
        }
    }

    pub fn error(id: &str, error: &str) -> Self {
        Self {
            id: id.into(),
            success: false,
            result: serde_json::Value::Null,
            error: Some(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wire_protocol_roundtrip() {
        let msg =
            FederationMessage::hello("peer-abc", "laptop", "0.1.0", PeerCapabilities::default());

        let json = serde_json::to_string(&msg).unwrap();
        let restored: FederationMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.method, "federation.hello");

        // Parse params back
        let params: HelloParams = serde_json::from_value(restored.params).unwrap();
        assert_eq!(params.peer_id, "peer-abc");
        assert_eq!(params.name, "laptop");
    }

    #[test]
    fn test_sync_message_roundtrip() {
        let changes = vec![SyncEntry {
            key: "test".into(),
            value: serde_json::json!(42),
            version: 1,
            timestamp: "2026-01-01T00:00:00Z".into(),
            origin_peer: "peer-a".into(),
        }];

        let msg = FederationMessage::sync(0, changes);
        let json = serde_json::to_string(&msg).unwrap();
        let restored: FederationMessage = serde_json::from_str(&json).unwrap();

        let params: SyncParams = serde_json::from_value(restored.params).unwrap();
        assert_eq!(params.changes.len(), 1);
        assert_eq!(params.since_version, 0);
    }

    #[test]
    fn test_skill_request_message() {
        let msg = FederationMessage::skill_request("skill-123");
        assert_eq!(msg.method, "federation.skill_request");
        let params: SkillRequestParams = serde_json::from_value(msg.params).unwrap();
        assert_eq!(params.skill_id, "skill-123");
    }

    #[test]
    fn test_hello_params_roundtrip() {
        let params = HelloParams {
            peer_id: "peer-1".into(),
            name: "test".into(),
            version: "1.0".into(),
            capabilities: PeerCapabilities::default(),
        };
        let json = serde_json::to_string(&params).unwrap();
        let restored: HelloParams = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.peer_id, "peer-1");
    }

    #[test]
    fn test_discover_params_roundtrip() {
        let params = DiscoverParams {
            query: "memory".into(),
        };
        let json = serde_json::to_string(&params).unwrap();
        let restored: DiscoverParams = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.query, "memory");
    }

    #[test]
    fn test_delegate_params_roundtrip() {
        let params = DelegateParams {
            task_id: "t-1".into(),
            description: "test".into(),
            requirements: vec!["mem".into(), "vis".into()],
        };
        let json = serde_json::to_string(&params).unwrap();
        let restored: DelegateParams = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.requirements.len(), 2);
    }

    #[test]
    fn test_skill_offer_params_roundtrip() {
        let params = SkillOfferParams {
            skill_id: "s-1".into(),
            name: "test_skill".into(),
            version: "1.0.0".into(),
            signature: "input->output".into(),
        };
        let json = serde_json::to_string(&params).unwrap();
        let restored: SkillOfferParams = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.signature, "input->output");
    }

    #[test]
    fn test_federation_message_serialization() {
        let msg = FederationMessage {
            id: "msg-1".into(),
            method: "test".into(),
            params: serde_json::json!({"key": "value"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let restored: FederationMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "msg-1");
        assert_eq!(restored.method, "test");
    }

    #[test]
    fn test_response_success_and_error() {
        let ok = FederationResponse::success("1", serde_json::json!({"status": "ok"}));
        assert!(ok.success);
        assert!(ok.error.is_none());

        let err = FederationResponse::error("2", "not found");
        assert!(!err.success);
        assert_eq!(err.error.as_deref(), Some("not found"));
    }
}
