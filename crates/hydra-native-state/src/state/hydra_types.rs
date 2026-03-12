//! Hydra state types — CognitivePhase, GlobeState, AppConfig, and supporting enums/structs.

use serde::{Deserialize, Serialize};

/// Cognitive phase in the loop
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitivePhase {
    Perceive,
    Think,
    Decide,
    Act,
    Learn,
}

impl CognitivePhase {
    pub const ALL: &'static [CognitivePhase] = &[
        Self::Perceive,
        Self::Think,
        Self::Decide,
        Self::Act,
        Self::Learn,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Perceive => "Perceive",
            Self::Think => "Think",
            Self::Decide => "Decide",
            Self::Act => "Act",
            Self::Learn => "Learn",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Self::Perceive => 0,
            Self::Think => 1,
            Self::Decide => 2,
            Self::Act => 3,
            Self::Learn => 4,
        }
    }
}

/// Status of a phase in the current run
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Phase tracking for the current run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseStatus {
    pub phase: CognitivePhase,
    pub state: PhaseState,
    pub tokens_used: Option<u64>,
    pub duration_ms: Option<u64>,
}

/// Voice globe animation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GlobeState {
    Idle,
    Listening,
    Processing,
    Speaking,
    Error,
    Approval,
}

impl GlobeState {
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Idle => "globe-idle",
            Self::Listening => "globe-listening",
            Self::Processing => "globe-processing",
            Self::Speaking => "globe-speaking",
            Self::Error => "globe-error",
            Self::Approval => "globe-approval",
        }
    }
}

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
    pub run_id: Option<String>,
    pub tokens_used: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Hydra,
}

/// A running or completed run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub intent: String,
    pub status: RunStatus,
    pub phases: Vec<PhaseStatus>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub total_tokens: Option<u64>,
    pub response: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server_url: String,
    pub theme: Theme,
    pub voice_enabled: bool,
    pub sounds_enabled: bool,
    pub sound_volume: f32,
    pub auto_approve_low_risk: bool,
    pub default_mode: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:3000".into(),
            theme: Theme::Dark,
            voice_enabled: false,
            sounds_enabled: true,
            sound_volume: 0.7,
            auto_approve_low_risk: false,
            default_mode: "companion".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    System,
}
