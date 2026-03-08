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
