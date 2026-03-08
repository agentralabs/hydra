use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::deployment::DeploymentStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HydraEvent {
    SessionStarted {
        session_id: Uuid,
    },
    IntentReceived {
        intent_id: Uuid,
        text: String,
    },
    IntentCompiled {
        intent_id: Uuid,
        confidence: f64,
    },
    DeploymentStarted {
        deployment_id: Uuid,
    },
    DeploymentProgress {
        deployment_id: Uuid,
        step: String,
        progress: f64,
    },
    DeploymentComplete {
        deployment_id: Uuid,
        status: DeploymentStatus,
    },
    ApprovalRequired {
        deployment_id: Uuid,
        reason: String,
    },
    SisterConnected {
        sister_name: String,
    },
    SisterDisconnected {
        sister_name: String,
        reason: String,
    },
    KernelStarted {
        version: String,
    },
    KernelShuttingDown {
        reason: String,
    },
    TokenBudgetWarning {
        remaining_percent: f64,
    },
    Error {
        source: String,
        message: String,
    },
}

impl HydraEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::SessionStarted { .. } => "session.started",
            Self::IntentReceived { .. } => "intent.received",
            Self::IntentCompiled { .. } => "intent.compiled",
            Self::DeploymentStarted { .. } => "deployment.started",
            Self::DeploymentProgress { .. } => "deployment.progress",
            Self::DeploymentComplete { .. } => "deployment.complete",
            Self::ApprovalRequired { .. } => "approval.required",
            Self::SisterConnected { .. } => "sister.connected",
            Self::SisterDisconnected { .. } => "sister.disconnected",
            Self::KernelStarted { .. } => "kernel.started",
            Self::KernelShuttingDown { .. } => "kernel.shutting_down",
            Self::TokenBudgetWarning { .. } => "token.budget_warning",
            Self::Error { .. } => "error",
        }
    }
}
