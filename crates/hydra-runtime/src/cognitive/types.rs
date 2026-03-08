use serde::{Deserialize, Serialize};

use hydra_core::types::CognitivePhase;

/// Result of the PERCEIVE phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Perception {
    pub intent: String,
    pub intent_type: String,
    pub entities: Vec<Entity>,
    pub implicit_context: Vec<String>,
    pub urgency: Urgency,
    pub required_sisters: Vec<String>,
}

impl Default for Perception {
    fn default() -> Self {
        Self {
            intent: String::new(),
            intent_type: "unknown".into(),
            entities: vec![],
            implicit_context: vec![],
            urgency: Urgency::Medium,
            required_sisters: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Low,
    Medium,
    High,
    Critical,
}

/// Result of the THINK phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingResult {
    pub reasoning: String,
    pub steps: Vec<String>,
    pub missing_info: Vec<String>,
    pub risks: Vec<String>,
    pub confidence: f64,
}

impl Default for ThinkingResult {
    fn default() -> Self {
        Self {
            reasoning: String::new(),
            steps: vec![],
            missing_info: vec![],
            risks: vec![],
            confidence: 0.5,
        }
    }
}

/// Result of the DECIDE phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub action: String,
    pub rationale: String,
    pub target: Option<String>,
    pub fallback: Option<String>,
    pub reversible: bool,
}

impl Default for Decision {
    fn default() -> Self {
        Self {
            action: "none".into(),
            rationale: String::new(),
            target: None,
            fallback: None,
            reversible: true,
        }
    }
}

/// Result of the LEARN phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningResult {
    pub summary: String,
    pub patterns_observed: Vec<String>,
    pub should_remember: bool,
}

impl Default for LearningResult {
    fn default() -> Self {
        Self {
            summary: String::new(),
            patterns_observed: vec![],
            should_remember: false,
        }
    }
}

/// SSE event emitted during cognitive phases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitivePhaseEvent {
    pub phase: CognitivePhase,
    pub status: PhaseEventStatus,
    pub tokens_used: u64,
    pub duration_ms: u64,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PhaseEventStatus {
    Started,
    Completed,
    Failed,
}
