//! Attribution cost classification.
//!
//! Maps settlement cost categories into richer attribution-level
//! cost classes with metadata for causal analysis.

use serde::{Deserialize, Serialize};

/// Detailed cost class for attribution analysis.
///
/// Extends settlement's flat `CostCategory` with metadata needed
/// for causal reasoning (e.g. how many rerouting attempts, which sister).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CostClass {
    /// Direct execution of the primary action.
    DirectExecution,
    /// Overhead from rerouting through alternative approaches.
    ReroutingOverhead {
        /// Number of rerouting attempts.
        attempts: u32,
    },
    /// Cost of calling a sister MCP server.
    SisterCallCost {
        /// Name of the sister that was called.
        sister_name: String,
    },
    /// Cost of acquiring new knowledge during execution.
    KnowledgeAcquisition {
        /// Topic of the knowledge acquired.
        topic: String,
    },
    /// Cost of red-team adversarial verification.
    RedTeamCost,
    /// Cost of wisdom synthesis and reflection.
    WisdomSynthesis,
    /// Cost of executing an Agentra skill action.
    SkillAction {
        /// Name of the skill that was executed.
        skill_name: String,
    },
}

impl CostClass {
    /// Return a human-readable label for this cost class.
    pub fn label(&self) -> &str {
        match self {
            Self::DirectExecution => "direct-execution",
            Self::ReroutingOverhead { .. } => "rerouting-overhead",
            Self::SisterCallCost { .. } => "sister-call",
            Self::KnowledgeAcquisition { .. } => "knowledge-acquisition",
            Self::RedTeamCost => "red-team",
            Self::WisdomSynthesis => "wisdom-synthesis",
            Self::SkillAction { .. } => "skill-action",
        }
    }
}

/// A single cost item for attribution analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostItem {
    /// The detailed cost class.
    pub class: CostClass,
    /// Number of tokens consumed by this cost.
    pub token_count: u64,
    /// Monetary amount of this cost.
    pub amount: f64,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

impl CostItem {
    /// Create a new attribution cost item.
    pub fn new(class: CostClass, token_count: u64, amount: f64, duration_ms: u64) -> Self {
        Self {
            class,
            token_count,
            amount,
            duration_ms,
        }
    }
}
