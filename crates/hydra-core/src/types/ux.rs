use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::kernel::RiskLevel;

/// The 6 types of proactive updates Hydra sends to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProactiveUpdate {
    Acknowledgment {
        message: String,
    },
    Progress {
        percent: f64,
        message: String,
        deployment_id: Option<Uuid>,
    },
    Event {
        title: String,
        detail: String,
    },
    Decision {
        request: DecisionRequest,
    },
    Completion {
        summary: CompletionSummary,
    },
    Alert {
        level: AlertLevel,
        message: String,
        suggestion: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
}

/// The 8 icon states for the living icon
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IconState {
    Idle,
    Listening,
    Working,
    NeedsAttention,
    ApprovalNeeded,
    Success,
    Error,
    Offline,
}

impl IconState {
    pub fn animation_description(&self) -> &'static str {
        match self {
            Self::Idle => "Soft glow, breathing animation",
            Self::Listening => "Pulsing",
            Self::Working => "Gentle spin",
            Self::NeedsAttention => "Orange pulse",
            Self::ApprovalNeeded => "Gentle bounce",
            Self::Success => "Green flash (2s)",
            Self::Error => "Red, still",
            Self::Offline => "Hollow ring",
        }
    }

    pub fn is_transient(&self) -> bool {
        matches!(self, Self::Success)
    }

    pub fn transient_duration_ms(&self) -> Option<u64> {
        match self {
            Self::Success => Some(2000),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRequest {
    pub id: Uuid,
    pub question: String,
    pub options: Vec<DecisionOption>,
    pub timeout_seconds: Option<u64>,
    pub default: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOption {
    pub label: String,
    pub description: Option<String>,
    pub risk_level: Option<RiskLevel>,
    pub keyboard_shortcut: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResponse {
    pub request_id: Uuid,
    pub chosen_option: usize,
    pub custom_input: Option<String>,
}

/// Completion summary sent to user after an action finishes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionSummary {
    pub headline: String,
    pub actions: Vec<String>,
    pub changes: Vec<String>,
    pub next_steps: Vec<String>,
}

/// Onboarding flow types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStep {
    Welcome,
    AskName,
    AskVoice,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingState {
    pub current_step: OnboardingStep,
    pub user_name: Option<String>,
    pub voice_enabled: Option<bool>,
    pub completed: bool,
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            current_step: OnboardingStep::Welcome,
            user_name: None,
            voice_enabled: None,
            completed: false,
        }
    }
}
