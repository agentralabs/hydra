use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

use super::kernel::RiskLevel;

// ── Raw Intent (pre-compilation) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentMetadata {
    pub source: IntentSource,
    pub session_id: Option<Uuid>,
    pub priority: Option<u8>,
    pub tags: Vec<String>,
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub id: Uuid,
    pub text: String,
    pub metadata: IntentMetadata,
    pub timestamp: DateTime<Utc>,
}

impl Intent {
    pub fn new(text: impl Into<String>, source: IntentSource) -> Self {
        Self {
            id: Uuid::new_v4(),
            text: text.into(),
            metadata: IntentMetadata {
                source,
                session_id: None,
                priority: None,
                tags: vec![],
                extra: HashMap::new(),
            },
            timestamp: Utc::now(),
        }
    }
}

// ── Intent Source ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentSource {
    Voice,
    Cli,
    Console,
    Api,
    Scheduled,
}

// ── Action ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Read,
    Write,
    Execute,
    Network,
    System,
    FileCreate,
    FileModify,
    FileDelete,
    ShellExecute,
    GitOperation,
    ApiCall,
    SisterCall,
    Composite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: Uuid,
    pub action_type: ActionType,
    pub target: String,
    pub params: serde_json::Value,
    pub risk: RiskLevel,
}

impl Action {
    pub fn new(action_type: ActionType, target: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            action_type,
            target: target.into(),
            params: serde_json::Value::Null,
            risk: RiskLevel::None,
        }
    }
}

// ── ActionResult ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideEffect {
    pub description: String,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub side_effects: Vec<SideEffect>,
    #[serde(with = "crate::types::duration_serde")]
    pub duration: Duration,
}

// ── Goal ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalType {
    Create,
    Modify,
    Delete,
    Query,
    Execute,
    Deploy,
    Debug,
    Review,
    Explain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub goal_type: GoalType,
    pub target: String,
    pub outcome: String,
    pub sub_goals: Vec<Goal>,
}

// ── Entity ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    FilePath,
    ModuleName,
    FunctionName,
    ClassName,
    VariableName,
    Url,
    PackageName,
    BranchName,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: Uuid,
    pub entity_type: EntityType,
    pub value: String,
    pub resolved_path: Option<PathBuf>,
    pub confidence: f64,
}

// ── Constraints & Criteria ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub constraint_type: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriterion {
    pub description: String,
    pub verifiable: bool,
    pub verification_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VeritasValidation {
    pub validated: bool,
    pub safety_score: f64,
    pub warnings: Vec<String>,
}

// ── Compiled Intent ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledIntent {
    pub id: Uuid,
    pub raw_text: String,
    pub source: IntentSource,
    pub goal: Goal,
    pub entities: Vec<Entity>,
    pub actions: Vec<Action>,
    pub constraints: Vec<Constraint>,
    pub success_criteria: Vec<SuccessCriterion>,
    pub confidence: f64,
    pub estimated_steps: usize,
    pub tokens_used: u64,
    pub veritas_validation: VeritasValidation,
}

impl CompiledIntent {
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.8
    }

    pub fn is_multi_step(&self) -> bool {
        self.estimated_steps > 1
    }

    pub fn has_destructive_actions(&self) -> bool {
        self.actions.iter().any(|a| {
            matches!(
                a.action_type,
                ActionType::FileDelete | ActionType::ShellExecute | ActionType::System
            )
        })
    }

    pub fn action_types(&self) -> Vec<&ActionType> {
        self.actions.iter().map(|a| &a.action_type).collect()
    }
}
