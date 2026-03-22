//! ConsensusResolution — the merged result of two conflicting beliefs.

use serde::{Deserialize, Serialize};

/// How the conflict was resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResolutionMethod {
    /// One belief clearly dominated — accepted with reduced confidence.
    DominantBelief { winner: String },
    /// Both partially right — synthesized with uncertainty.
    Synthesis,
    /// Neither could be resolved — flagged for principal.
    Unresolvable,
}

impl ResolutionMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::DominantBelief { .. } => "dominant",
            Self::Synthesis => "synthesis",
            Self::Unresolvable => "unresolvable",
        }
    }
}

/// The consensus result for one topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResolution {
    pub id: String,
    pub topic: String,
    pub merged_claim: String,
    pub merged_confidence: f64,
    pub method: ResolutionMethod,
    pub is_uncertain: bool,
    pub provenance: Vec<String>,
    pub resolved_at: chrono::DateTime<chrono::Utc>,
}

impl ConsensusResolution {
    pub fn new(
        topic: impl Into<String>,
        merged_claim: impl Into<String>,
        merged_confidence: f64,
        method: ResolutionMethod,
        provenance: Vec<String>,
    ) -> Self {
        let conf = merged_confidence.clamp(0.0, 1.0);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic: topic.into(),
            merged_claim: merged_claim.into(),
            merged_confidence: conf,
            is_uncertain: conf < crate::constants::MIN_MERGED_CONFIDENCE,
            method,
            provenance,
            resolved_at: chrono::Utc::now(),
        }
    }

    pub fn is_resolved(&self) -> bool {
        !matches!(self.method, ResolutionMethod::Unresolvable)
    }
}
