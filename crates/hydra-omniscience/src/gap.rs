//! KnowledgeGap — what Hydra doesn't know and needs to find out.
//! Every gap is detected, tracked, and either closed or escalated.

use crate::constants::RECURRING_GAP_THRESHOLD;
use serde::{Deserialize, Serialize};

/// The type of knowledge gap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GapType {
    /// Missing factual knowledge about a concept or system.
    Factual { domain: String },
    /// Missing procedural knowledge — how to do something.
    Procedural { action: String },
    /// Missing contextual knowledge — what is the current state.
    Contextual { system: String },
    /// Missing structural knowledge — how things connect.
    Structural { relationship: String },
    /// API or protocol specification unknown.
    ApiSpec { service: String },
    /// Domain vocabulary not loaded — needs a skill.
    VocabularyMissing { domain: String },
}

impl GapType {
    pub fn label(&self) -> String {
        match self {
            Self::Factual { domain }          => format!("factual:{}", domain),
            Self::Procedural { action }       => format!("procedural:{}", action),
            Self::Contextual { system }       => format!("contextual:{}", system),
            Self::Structural { relationship } => format!("structural:{}", relationship),
            Self::ApiSpec { service }         => format!("api-spec:{}", service),
            Self::VocabularyMissing { domain } => format!("vocab-missing:{}", domain),
        }
    }

    /// Does this gap type require skill loading rather than acquisition?
    pub fn needs_skill_load(&self) -> bool {
        matches!(self, Self::VocabularyMissing { .. })
    }
}

/// The state of a knowledge gap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GapState {
    /// Detected — acquisition plan not yet built.
    Detected,
    /// Acquisition in progress.
    Acquiring { source: String },
    /// Successfully closed — knowledge integrated.
    Closed { confidence: f64, source: String },
    /// Could not be resolved — needs human.
    Escalated { reason: String },
    /// Deferred — not critical right now.
    Deferred,
}

impl GapState {
    pub fn is_resolved(&self) -> bool {
        matches!(self, Self::Closed { .. } | Self::Escalated { .. })
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Detected        => "detected",
            Self::Acquiring { .. } => "acquiring",
            Self::Closed { .. }   => "closed",
            Self::Escalated { .. } => "escalated",
            Self::Deferred        => "deferred",
        }
    }
}

/// One knowledge gap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGap {
    pub id:           String,
    pub topic:        String,
    pub gap_type:     GapType,
    pub state:        GapState,
    pub recurrence:   usize,
    pub priority:     f64,
    pub detected_at:  chrono::DateTime<chrono::Utc>,
    pub resolved_at:  Option<chrono::DateTime<chrono::Utc>>,
}

impl KnowledgeGap {
    pub fn new(
        topic:    impl Into<String>,
        gap_type: GapType,
        priority: f64,
    ) -> Self {
        Self {
            id:           uuid::Uuid::new_v4().to_string(),
            topic:        topic.into(),
            gap_type,
            state:        GapState::Detected,
            recurrence:   1,
            priority:     priority.clamp(0.0, 1.0),
            detected_at:  chrono::Utc::now(),
            resolved_at:  None,
        }
    }

    pub fn increment_recurrence(&mut self) {
        self.recurrence += 1;
    }

    pub fn is_recurring(&self) -> bool {
        self.recurrence >= RECURRING_GAP_THRESHOLD
    }

    pub fn close(&mut self, confidence: f64, source: &str) {
        self.state = GapState::Closed {
            confidence,
            source: source.to_string(),
        };
        self.resolved_at = Some(chrono::Utc::now());
    }

    pub fn escalate(&mut self, reason: &str) {
        self.state = GapState::Escalated {
            reason: reason.to_string(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_gap_is_detected() {
        let g = KnowledgeGap::new(
            "kubernetes rolling update",
            GapType::Procedural { action: "rolling-update".into() },
            0.8,
        );
        assert_eq!(g.state.label(), "detected");
        assert_eq!(g.recurrence, 1);
    }

    #[test]
    fn recurrence_triggers_flag() {
        let mut g = KnowledgeGap::new(
            "test topic",
            GapType::Factual { domain: "test".into() },
            0.5,
        );
        assert!(!g.is_recurring());
        for _ in 1..RECURRING_GAP_THRESHOLD {
            g.increment_recurrence();
        }
        assert!(g.is_recurring());
    }

    #[test]
    fn close_marks_resolved() {
        let mut g = KnowledgeGap::new(
            "test", GapType::Factual { domain: "d".into() }, 0.5,
        );
        g.close(0.85, "codebase");
        assert!(g.state.is_resolved());
        assert_eq!(g.state.label(), "closed");
        assert!(g.resolved_at.is_some());
    }

    #[test]
    fn vocab_gap_needs_skill_load() {
        let g = KnowledgeGap::new(
            "video editing vocabulary",
            GapType::VocabularyMissing { domain: "video".into() },
            0.7,
        );
        assert!(g.gap_type.needs_skill_load());
    }
}
