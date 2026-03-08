use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolType {
    RestApi,
    Sister,
    McpTool,
    Shell,
    FileSystem,
    Git,
    Database,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub url: Option<String>,
    pub command: Option<String>,
    pub sister_name: Option<String>,
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthRequirement {
    None,
    ApiKey,
    Bearer,
    Certificate,
    SisterAuth,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtocolMetrics {
    pub calls_total: u64,
    pub calls_success: u64,
    pub avg_latency_ms: f64,
    pub last_error: Option<String>,
    pub uptime_ratio: f64,
}

/// Full protocol info (data structure, used in registries and rankings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolInfo {
    pub id: Uuid,
    pub protocol_type: ProtocolType,
    pub name: String,
    pub endpoint: Endpoint,
    pub auth: AuthRequirement,
    pub metrics: ProtocolMetrics,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterMapping {
    pub mappings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedProtocol {
    pub protocol: ProtocolInfo,
    pub score: f64,
    pub rank: usize,
    pub parameter_mapping: ParameterMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualGuidance {
    pub instructions: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedProtocols {
    pub intent_id: Uuid,
    pub primary: RankedProtocol,
    pub fallbacks: Vec<RankedProtocol>,
    pub manual_fallback: ManualGuidance,
}
