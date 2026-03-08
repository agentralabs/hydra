use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use super::identity::HydraSession;
use super::intent::Goal;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KernelStatus {
    Starting,
    Running,
    Degraded,
    ShuttingDown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SisterStatus {
    Connected,
    Disconnected,
    Degraded,
    NotConfigured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SisterConnection {
    pub name: String,
    pub status: SisterStatus,
    pub endpoint: String,
    pub version: Option<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentState {
    Compiling,
    Hunting,
    Gating,
    Executing,
    Learning,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KernelMetrics {
    #[serde(with = "crate::types::duration_serde")]
    pub uptime: Duration,
    pub requests_total: u64,
    pub deployments_total: u64,
    pub deployments_success: u64,
    pub memory_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelState {
    pub id: Uuid,
    pub status: KernelStatus,
    pub active_sessions: HashMap<Uuid, HydraSession>,
    pub active_deployments: HashMap<Uuid, DeploymentState>,
    pub sister_connections: HashMap<String, SisterConnection>,
    pub metrics: KernelMetrics,
}

// ── Cognitive Loop types ──

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CognitivePhase {
    Perceive,
    Think,
    Decide,
    Act,
    Learn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub key: String,
    pub value: serde_json::Value,
    pub confidence: f64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveState {
    pub phase: CognitivePhase,
    pub intent_id: Option<Uuid>,
    pub context: serde_json::Value,
    pub goals: Vec<Goal>,
    pub budget: TokenBudget,
    pub beliefs: Vec<Belief>,
    pub checkpoint: Option<serde_json::Value>,
}

// ── Token Budget ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    pub total: u64,
    pub remaining: u64,
    pub per_phase: HashMap<CognitivePhase, u64>,
    pub conservation_mode: bool,
}

impl TokenBudget {
    pub fn new(total: u64) -> Self {
        let per_phase_alloc = total / 5;
        let mut per_phase = HashMap::new();
        per_phase.insert(CognitivePhase::Perceive, per_phase_alloc);
        per_phase.insert(CognitivePhase::Think, per_phase_alloc * 2);
        per_phase.insert(CognitivePhase::Decide, per_phase_alloc);
        per_phase.insert(CognitivePhase::Act, per_phase_alloc);
        per_phase.insert(CognitivePhase::Learn, total - per_phase_alloc * 5);

        let conservation_mode = total == 0;
        Self {
            total,
            remaining: total,
            per_phase,
            conservation_mode,
        }
    }

    pub fn can_afford(&self, tokens: u64) -> bool {
        self.remaining >= tokens
    }

    pub fn record_usage(&mut self, tokens: u64) {
        self.remaining = self.remaining.saturating_sub(tokens);
        self.conservation_mode = self.is_below_threshold();
    }

    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.remaining)
    }

    pub fn is_below_threshold(&self) -> bool {
        if self.total == 0 {
            return true;
        }
        (self.remaining as f64 / self.total as f64) < 0.25
    }

    pub fn utilization(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.used() as f64 / self.total as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenMetrics {
    pub used: u64,
    pub cached_hits: u64,
    pub llm_calls: u64,
    pub efficiency: f64,
    pub tokens_saved_by_batching: u64,
}

// ── Risk Assessment ──

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub level: RiskLevel,
    pub factors: Vec<RiskFactor>,
    pub mitigations: Vec<String>,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub name: String,
    pub severity: RiskLevel,
    pub description: String,
}

impl RiskAssessment {
    pub fn needs_approval(&self) -> bool {
        self.requires_approval || self.level >= RiskLevel::High
    }
}
