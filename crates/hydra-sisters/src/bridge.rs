use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Sister identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SisterId {
    // Foundation (7)
    Memory,
    Vision,
    Codebase,
    Identity,
    Time,
    Contract,
    Comm,
    // Cognitive (3)
    Planning,
    Cognition,
    Reality,
    // Astral (4)
    Forge,
    Aegis,
    Veritas,
    Evolve,
}

impl SisterId {
    pub fn all() -> &'static [SisterId] {
        &[
            Self::Memory,
            Self::Vision,
            Self::Codebase,
            Self::Identity,
            Self::Time,
            Self::Contract,
            Self::Comm,
            Self::Planning,
            Self::Cognition,
            Self::Reality,
            Self::Forge,
            Self::Aegis,
            Self::Veritas,
            Self::Evolve,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Memory => "agentic-memory",
            Self::Vision => "agentic-vision",
            Self::Codebase => "agentic-codebase",
            Self::Identity => "agentic-identity",
            Self::Time => "agentic-time",
            Self::Contract => "agentic-contract",
            Self::Comm => "agentic-comm",
            Self::Planning => "agentic-planning",
            Self::Cognition => "agentic-cognition",
            Self::Reality => "agentic-reality",
            Self::Forge => "agentic-forge",
            Self::Aegis => "agentic-aegis",
            Self::Veritas => "agentic-veritas",
            Self::Evolve => "agentic-evolve",
        }
    }

    pub fn is_foundation(&self) -> bool {
        matches!(
            self,
            Self::Memory
                | Self::Vision
                | Self::Codebase
                | Self::Identity
                | Self::Time
                | Self::Contract
                | Self::Comm
        )
    }

    pub fn is_cognitive(&self) -> bool {
        matches!(self, Self::Planning | Self::Cognition | Self::Reality)
    }

    pub fn is_astral(&self) -> bool {
        matches!(
            self,
            Self::Forge | Self::Aegis | Self::Veritas | Self::Evolve
        )
    }
}

/// Action to send to a sister
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SisterAction {
    pub tool: String,
    pub params: serde_json::Value,
}

impl SisterAction {
    pub fn new(tool: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            tool: tool.into(),
            params,
        }
    }
}

/// Result from a sister call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SisterResult {
    pub data: serde_json::Value,
    pub tokens_used: u64,
}

/// Sister-specific error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SisterError {
    pub sister_id: SisterId,
    pub message: String,
    pub retryable: bool,
}

impl std::fmt::Display for SisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} error: {}. {}",
            self.sister_id.name(),
            self.message,
            if self.retryable {
                "This may be temporary."
            } else {
                "Check sister status."
            }
        )
    }
}

impl std::error::Error for SisterError {}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod tests;

/// Health status from a sister
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

/// The sister bridge trait — all 17 sisters implement this
#[async_trait]
pub trait SisterBridge: Send + Sync {
    fn sister_id(&self) -> SisterId;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    async fn health_check(&self) -> HealthStatus;
    async fn call(&self, action: SisterAction) -> Result<SisterResult, SisterError>;
    async fn batch_call(
        &self,
        actions: Vec<SisterAction>,
    ) -> Vec<Result<SisterResult, SisterError>>;
    fn capabilities(&self) -> Vec<String>;
}
