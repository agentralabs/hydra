use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::error::HydraError;
use crate::types::{
    CompiledIntent, HydraEvent, IconState, ProactiveUpdate, RankedProtocols, RiskAssessment,
};

/// Core trait representing a sister module's identity and status
#[async_trait]
pub trait Sister: Send + Sync {
    /// Sister name identifier
    fn name(&self) -> &str;

    /// Sister version
    fn version(&self) -> &str;

    /// Check if the sister is healthy/available
    async fn health_check(&self) -> Result<bool, HydraError>;

    /// List capabilities this sister provides
    fn capabilities(&self) -> Vec<String>;
}

/// Trait for sister module communication bridges (tool invocation)
#[async_trait]
pub trait SisterBridge: Sister {
    /// List available tools
    async fn list_tools(&self) -> Result<Vec<String>, HydraError>;

    /// Call a tool on the sister
    async fn call_tool(&self, tool: &str, params: Value) -> Result<Value, HydraError>;

    /// Batch multiple tool calls for token conservation
    async fn batch_call(&self, calls: Vec<(&str, Value)>) -> Result<Vec<Value>, HydraError> {
        let mut results = Vec::with_capacity(calls.len());
        for (tool, params) in calls {
            results.push(self.call_tool(tool, params).await?);
        }
        Ok(results)
    }
}

/// Trait representing a protocol's identity and metadata
pub trait Protocol: Send + Sync {
    /// Protocol name
    fn name(&self) -> &str;

    /// Protocol type identifier
    fn protocol_type(&self) -> &str;

    /// Whether the protocol is currently available
    fn is_available(&self) -> bool;

    /// Health/uptime metrics
    fn uptime_ratio(&self) -> f64;
}

/// Trait for protocol execution
#[async_trait]
pub trait ProtocolHandler: Protocol {
    /// Execute an action via this protocol
    async fn execute(&self, intent: &CompiledIntent, params: Value) -> Result<Value, HydraError>;

    /// Check if this protocol can handle the given intent
    fn can_handle(&self, intent: &CompiledIntent) -> bool;
}

/// Trait for the intent compiler
#[async_trait]
pub trait IntentCompiler: Send + Sync {
    /// Compile raw text into a structured intent
    async fn compile(&self, text: &str) -> Result<CompiledIntent, HydraError>;

    /// Get cache hit rate
    fn cache_hit_rate(&self) -> f64;
}

/// Trait for the protocol hunter
#[async_trait]
pub trait ProtocolHunter: Send + Sync {
    /// Find and rank protocols for a compiled intent
    async fn hunt(&self, intent: &CompiledIntent) -> Result<RankedProtocols, HydraError>;

    /// Register a new protocol
    async fn register(&self, protocol: crate::types::ProtocolInfo) -> Result<(), HydraError>;
}

/// Trait for the execution gate
#[async_trait]
pub trait ExecutionGate: Send + Sync {
    /// Evaluate risk for a set of actions
    async fn evaluate(&self, intent: &CompiledIntent) -> Result<RiskAssessment, HydraError>;
}

/// Trait for the receipt ledger
#[async_trait]
pub trait ReceiptLedger: Send + Sync {
    /// Record a receipt
    async fn record(&self, receipt: crate::types::Receipt) -> Result<(), HydraError>;

    /// Verify the chain integrity
    async fn verify_chain(&self) -> Result<bool, HydraError>;

    /// Get receipts for a deployment
    async fn get_by_deployment(
        &self,
        deployment_id: Uuid,
    ) -> Result<Vec<crate::types::Receipt>, HydraError>;
}

/// Trait for ambient monitoring
#[async_trait]
pub trait Monitor: Send + Sync {
    /// Start monitoring
    async fn start(&self) -> Result<(), HydraError>;

    /// Stop monitoring
    async fn stop(&self) -> Result<(), HydraError>;

    /// Check for proactive suggestions
    async fn check_suggestions(&self) -> Result<Vec<ProactiveUpdate>, HydraError>;
}

/// Trait for UX event emission
#[async_trait]
pub trait UxEmitter: Send + Sync {
    /// Emit a proactive update to the user
    async fn emit(&self, update: ProactiveUpdate) -> Result<(), HydraError>;

    /// Update the icon state
    async fn set_icon_state(&self, state: IconState) -> Result<(), HydraError>;

    /// Subscribe to events
    async fn subscribe(&self) -> Result<tokio::sync::broadcast::Receiver<HydraEvent>, HydraError>;
}
