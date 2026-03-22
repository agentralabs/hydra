//! AcquisitionPlan — the ordered strategy for closing a knowledge gap.

use crate::{
    gap::KnowledgeGap,
    source::AcquisitionSource,
};
use serde::{Deserialize, Serialize};

/// The acquisition plan for one gap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionPlan {
    pub gap_id:     String,
    pub topic:      String,
    pub sources:    Vec<AcquisitionSource>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl AcquisitionPlan {
    pub fn for_gap(gap: &KnowledgeGap) -> Self {
        let domain = gap.gap_type.label();
        let domain_part = domain.split(':').nth(1).unwrap_or(&domain);
        let sources = AcquisitionSource::for_gap(&gap.topic, domain_part);

        Self {
            gap_id:     gap.id.clone(),
            topic:      gap.topic.clone(),
            sources,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn source_count(&self) -> usize { self.sources.len() }

    /// The highest-reliability source in this plan.
    pub fn best_source(&self) -> Option<&AcquisitionSource> {
        self.sources.iter()
            .max_by(|a, b| a.reliability().partial_cmp(&b.reliability())
                .unwrap_or(std::cmp::Ordering::Equal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gap::{GapType, KnowledgeGap};

    #[test]
    fn plan_created_for_gap() {
        let gap = KnowledgeGap::new(
            "kubernetes rolling update deployment",
            GapType::Procedural { action: "rolling-update".into() },
            0.8,
        );
        let plan = AcquisitionPlan::for_gap(&gap);
        assert!(!plan.sources.is_empty());
        assert_eq!(plan.gap_id, gap.id);
    }

    #[test]
    fn plan_has_best_source() {
        let gap = KnowledgeGap::new(
            "kubernetes api spec",
            GapType::ApiSpec { service: "k8s".into() },
            0.7,
        );
        let plan = AcquisitionPlan::for_gap(&gap);
        assert!(plan.best_source().is_some());
    }
}
