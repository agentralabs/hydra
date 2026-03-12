use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use hydra_core::error::HydraError;

use crate::health::HealthStatus;

/// Protocol kind with associated token cost estimates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolKind {
    Sister,
    ShellCommand,
    McpTool,
    RestApi,
    BrowserAutomation,
    LlmAgent,
}

impl ProtocolKind {
    /// Estimated token cost for this protocol kind
    pub fn token_cost(&self) -> u64 {
        match self {
            Self::Sister => 100,
            Self::ShellCommand => 50,
            Self::McpTool => 200,
            Self::RestApi => 500,
            Self::BrowserAutomation => 2000,
            Self::LlmAgent => 5000,
        }
    }
}

/// A registered protocol entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolEntry {
    pub id: Uuid,
    pub name: String,
    pub kind: ProtocolKind,
    pub description: String,
    pub capabilities: Vec<String>,
    pub available: bool,
    pub version: Option<String>,
    pub auth_required: bool,
    pub auth_valid: bool,
    pub registered_at: DateTime<Utc>,
    /// Reliability score 0.0–1.0 (from historical success rate)
    pub reliability: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Safety score 0.0–1.0
    pub safety: f64,
    /// Dependencies on other protocol IDs
    pub depends_on: Vec<Uuid>,
}

impl ProtocolEntry {
    pub fn new(name: impl Into<String>, kind: ProtocolKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            kind,
            description: String::new(),
            capabilities: vec![],
            available: true,
            version: None,
            auth_required: false,
            auth_valid: true,
            registered_at: Utc::now(),
            reliability: 1.0,
            avg_latency_ms: 100.0,
            safety: 1.0,
            depends_on: vec![],
        }
    }

    pub fn with_capabilities(mut self, caps: Vec<&str>) -> Self {
        self.capabilities = caps.into_iter().map(String::from).collect();
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_auth(mut self, required: bool) -> Self {
        self.auth_required = required;
        self.auth_valid = !required; // Need to authenticate first
        self
    }

    pub fn with_dependency(mut self, dep: Uuid) -> Self {
        self.depends_on.push(dep);
        self
    }

    /// Token cost for this protocol
    pub fn token_cost(&self) -> u64 {
        self.kind.token_cost()
    }

    /// Efficiency score: capability / token_cost (higher = better)
    pub fn efficiency_score(&self) -> f64 {
        let cost = self.token_cost() as f64;
        if cost == 0.0 {
            return 0.0;
        }
        // Weighted: reliability(0.35) × speed(0.25) × cost(0.20) × safety(0.20)
        let speed = 1.0 / (1.0 + self.avg_latency_ms / 1000.0); // normalize latency
        let cost_score = 1.0 / (1.0 + cost / 1000.0); // cheaper = higher
        self.reliability * 0.35 + speed * 0.25 + cost_score * 0.20 + self.safety * 0.20
    }

    /// Check if this protocol can handle the given capability
    pub fn can_handle(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    /// Check if protocol is usable (available + auth valid)
    pub fn is_usable(&self) -> bool {
        self.available && (!self.auth_required || self.auth_valid)
    }
}

/// Trait for executable protocol implementations
#[async_trait]
pub trait Protocol: Send + Sync {
    /// Protocol name
    fn name(&self) -> &str;

    /// Protocol type/kind
    fn protocol_type(&self) -> ProtocolKind;

    /// Whether the protocol is currently available
    fn is_available(&self) -> bool;

    /// Estimated token cost for this protocol
    fn token_cost(&self) -> u64 {
        self.protocol_type().token_cost()
    }

    /// Check health of this protocol
    async fn health(&self) -> HealthStatus;

    /// Execute an action via this protocol
    async fn execute(
        &self,
        action: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, HydraError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_kind_token_cost_sister() {
        assert_eq!(ProtocolKind::Sister.token_cost(), 100);
    }

    #[test]
    fn test_protocol_kind_token_cost_shell() {
        assert_eq!(ProtocolKind::ShellCommand.token_cost(), 50);
    }

    #[test]
    fn test_protocol_kind_token_cost_mcp() {
        assert_eq!(ProtocolKind::McpTool.token_cost(), 200);
    }

    #[test]
    fn test_protocol_kind_token_cost_rest() {
        assert_eq!(ProtocolKind::RestApi.token_cost(), 500);
    }

    #[test]
    fn test_protocol_kind_token_cost_browser() {
        assert_eq!(ProtocolKind::BrowserAutomation.token_cost(), 2000);
    }

    #[test]
    fn test_protocol_kind_token_cost_llm() {
        assert_eq!(ProtocolKind::LlmAgent.token_cost(), 5000);
    }

    #[test]
    fn test_protocol_entry_new() {
        let entry = ProtocolEntry::new("test-protocol", ProtocolKind::Sister);
        assert_eq!(entry.name, "test-protocol");
        assert_eq!(entry.kind, ProtocolKind::Sister);
        assert!(entry.available);
        assert!(!entry.auth_required);
        assert!(entry.auth_valid);
        assert_eq!(entry.reliability, 1.0);
        assert_eq!(entry.safety, 1.0);
        assert!(entry.depends_on.is_empty());
        assert!(entry.capabilities.is_empty());
    }

    #[test]
    fn test_protocol_entry_with_capabilities() {
        let entry = ProtocolEntry::new("test", ProtocolKind::McpTool)
            .with_capabilities(vec!["read", "write"]);
        assert_eq!(entry.capabilities.len(), 2);
        assert!(entry.capabilities.contains(&"read".to_string()));
    }

    #[test]
    fn test_protocol_entry_with_description() {
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister)
            .with_description("A test protocol");
        assert_eq!(entry.description, "A test protocol");
    }

    #[test]
    fn test_protocol_entry_with_version() {
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister)
            .with_version("1.0.0");
        assert_eq!(entry.version, Some("1.0.0".into()));
    }

    #[test]
    fn test_protocol_entry_with_auth_required() {
        let entry = ProtocolEntry::new("test", ProtocolKind::RestApi)
            .with_auth(true);
        assert!(entry.auth_required);
        assert!(!entry.auth_valid); // Need to authenticate first
    }

    #[test]
    fn test_protocol_entry_with_auth_not_required() {
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister)
            .with_auth(false);
        assert!(!entry.auth_required);
        assert!(entry.auth_valid);
    }

    #[test]
    fn test_protocol_entry_with_dependency() {
        let dep_id = uuid::Uuid::new_v4();
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister)
            .with_dependency(dep_id);
        assert_eq!(entry.depends_on.len(), 1);
        assert_eq!(entry.depends_on[0], dep_id);
    }

    #[test]
    fn test_protocol_entry_token_cost() {
        let entry = ProtocolEntry::new("test", ProtocolKind::RestApi);
        assert_eq!(entry.token_cost(), 500);
    }

    #[test]
    fn test_protocol_entry_efficiency_score() {
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister);
        let score = entry.efficiency_score();
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_protocol_entry_efficiency_score_higher_reliability() {
        let mut high = ProtocolEntry::new("high", ProtocolKind::Sister);
        high.reliability = 1.0;
        let mut low = ProtocolEntry::new("low", ProtocolKind::Sister);
        low.reliability = 0.1;
        assert!(high.efficiency_score() > low.efficiency_score());
    }

    #[test]
    fn test_protocol_entry_can_handle() {
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister)
            .with_capabilities(vec!["memory", "vision"]);
        assert!(entry.can_handle("memory"));
        assert!(entry.can_handle("vision"));
        assert!(!entry.can_handle("codebase"));
    }

    #[test]
    fn test_protocol_entry_is_usable_available_no_auth() {
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister);
        assert!(entry.is_usable());
    }

    #[test]
    fn test_protocol_entry_is_usable_unavailable() {
        let mut entry = ProtocolEntry::new("test", ProtocolKind::Sister);
        entry.available = false;
        assert!(!entry.is_usable());
    }

    #[test]
    fn test_protocol_entry_is_usable_auth_required_not_valid() {
        let entry = ProtocolEntry::new("test", ProtocolKind::RestApi)
            .with_auth(true);
        assert!(!entry.is_usable());
    }

    #[test]
    fn test_protocol_entry_is_usable_auth_required_valid() {
        let mut entry = ProtocolEntry::new("test", ProtocolKind::RestApi)
            .with_auth(true);
        entry.auth_valid = true;
        assert!(entry.is_usable());
    }

    #[test]
    fn test_protocol_kind_serialization() {
        let json = serde_json::to_string(&ProtocolKind::Sister).unwrap();
        assert_eq!(json, "\"sister\"");

        let json = serde_json::to_string(&ProtocolKind::BrowserAutomation).unwrap();
        assert_eq!(json, "\"browser_automation\"");
    }

    #[test]
    fn test_protocol_kind_deserialization() {
        let k: ProtocolKind = serde_json::from_str("\"shell_command\"").unwrap();
        assert_eq!(k, ProtocolKind::ShellCommand);
    }

    #[test]
    fn test_protocol_entry_serialization_roundtrip() {
        let entry = ProtocolEntry::new("test-proto", ProtocolKind::McpTool)
            .with_capabilities(vec!["tool1", "tool2"])
            .with_version("2.0.0");
        let json = serde_json::to_string(&entry).unwrap();
        let restored: ProtocolEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test-proto");
        assert_eq!(restored.kind, ProtocolKind::McpTool);
        assert_eq!(restored.capabilities.len(), 2);
        assert_eq!(restored.version, Some("2.0.0".into()));
    }

    #[test]
    fn test_protocol_entry_can_handle_empty_capabilities() {
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister);
        assert!(!entry.can_handle("anything"));
    }

    #[test]
    fn test_efficiency_lower_latency_higher_score() {
        let mut fast = ProtocolEntry::new("fast", ProtocolKind::Sister);
        fast.avg_latency_ms = 10.0;
        let mut slow = ProtocolEntry::new("slow", ProtocolKind::Sister);
        slow.avg_latency_ms = 10000.0;
        assert!(fast.efficiency_score() > slow.efficiency_score());
    }
}
