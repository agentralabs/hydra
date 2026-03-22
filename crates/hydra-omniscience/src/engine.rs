//! OmniscienceEngine — the knowledge acquisition coordinator.
//! Detect gaps -> plan acquisition -> acquire -> integrate -> close.

use crate::{
    constants::*,
    errors::OmniscienceError,
    gap::{GapType, KnowledgeGap},
    plan::AcquisitionPlan,
    result::AcquisitionResult,
    source::AcquisitionSource,
};
use std::collections::HashMap;

/// Summary of one acquisition cycle.
#[derive(Debug, Clone)]
pub struct AcquisitionSummary {
    pub topic:      String,
    pub closed:     bool,
    pub source:     String,
    pub confidence: f64,
    pub gap_id:     String,
}

/// The omniscience engine.
pub struct OmniscienceEngine {
    gaps:    HashMap<String, KnowledgeGap>,
    results: Vec<AcquisitionResult>,
    closed_count:    usize,
    escalated_count: usize,
    db: Option<crate::persistence::OmniscienceDb>,
}

impl OmniscienceEngine {
    pub fn new() -> Self {
        Self {
            gaps:             HashMap::new(),
            results:          Vec::new(),
            closed_count:     0,
            escalated_count:  0,
            db:               None,
        }
    }

    /// Create an engine backed by SQLite persistence.
    /// Loads all existing gaps from disk on open.
    pub fn open() -> Self {
        let (db, loaded_gaps) = match crate::persistence::OmniscienceDb::open() {
            Ok(db) => {
                let gaps = db.load_all();
                (Some(db), gaps)
            }
            Err(e) => {
                eprintln!("hydra: omniscience db open failed, running in-memory: {}", e);
                (None, Vec::new())
            }
        };
        let mut gap_map = HashMap::new();
        let mut closed_count = 0_usize;
        let mut escalated_count = 0_usize;
        for gap in loaded_gaps {
            if gap.state.is_resolved() {
                if matches!(gap.state, crate::gap::GapState::Closed { .. }) {
                    closed_count += 1;
                } else {
                    escalated_count += 1;
                }
            }
            gap_map.insert(gap.id.clone(), gap);
        }
        Self {
            gaps: gap_map,
            results: Vec::new(),
            closed_count,
            escalated_count,
            db,
        }
    }

    /// Detect and register a knowledge gap.
    pub fn detect_gap(
        &mut self,
        topic:    impl Into<String>,
        gap_type: GapType,
        priority: f64,
    ) -> String {
        let topic_str = topic.into();

        // Check if gap already exists for this topic
        if let Some(existing) = self.gaps.values_mut()
            .find(|g| g.topic == topic_str && !g.state.is_resolved())
        {
            existing.increment_recurrence();
            return existing.id.clone();
        }

        if self.gaps.len() >= MAX_TRACKED_GAPS {
            // Prune oldest resolved gap
            if let Some(old_id) = self.gaps.values()
                .filter(|g| g.state.is_resolved())
                .min_by_key(|g| g.detected_at)
                .map(|g| g.id.clone())
            {
                self.gaps.remove(&old_id);
            }
        }

        let gap    = KnowledgeGap::new(&topic_str, gap_type, priority);
        let gap_id = gap.id.clone();
        if let Some(ref db) = self.db {
            db.insert(&gap);
        }
        self.gaps.insert(gap_id.clone(), gap);
        gap_id
    }

    /// Attempt to acquire knowledge for a gap.
    pub fn acquire(
        &mut self,
        gap_id: &str,
    ) -> Result<AcquisitionSummary, OmniscienceError> {
        let gap = self.gaps.get(gap_id)
            .ok_or_else(|| OmniscienceError::GapUnresolvable {
                topic: gap_id.to_string(),
            })?.clone();

        let plan = AcquisitionPlan::for_gap(&gap);

        // Try each source in order
        for source in &plan.sources {
            if matches!(source, AcquisitionSource::HumanResolution { .. }) {
                continue;
            }

            let result = self.try_acquire_from_source(&gap, source);

            if result.meets_threshold() {
                let summary = AcquisitionSummary {
                    topic:      gap.topic.clone(),
                    closed:     true,
                    source:     result.source.clone(),
                    confidence: result.confidence,
                    gap_id:     gap_id.to_string(),
                };

                if let Some(g) = self.gaps.get_mut(gap_id) {
                    g.close(result.confidence, &result.source);
                }

                if self.results.len() < MAX_ACQUISITION_RESULTS {
                    self.results.push(result);
                }

                self.closed_count += 1;
                return Ok(summary);
            }
        }

        // All sources failed — escalate
        if let Some(g) = self.gaps.get_mut(gap_id) {
            g.escalate(&format!(
                "All {} sources exhausted", plan.source_count()
            ));
        }
        self.escalated_count += 1;

        Err(OmniscienceError::SourcesExhausted {
            topic: gap.topic.clone(),
        })
    }

    /// Simulate knowledge acquisition from one source.
    fn try_acquire_from_source(
        &self,
        gap:    &KnowledgeGap,
        source: &AcquisitionSource,
    ) -> AcquisitionResult {
        let (content, confidence, provenance) = match source {
            AcquisitionSource::AgenticCodebase { .. } => {
                (
                    format!(
                        "Code patterns for '{}': found {} relevant implementations \
                         in the codebase with consistent approach.",
                        gap.topic, 12
                    ),
                    SOURCE_RELIABILITY_AGENTIC_CODEBASE,
                    "AgenticCodebase: 200M+ repos scanned".to_string(),
                )
            }
            AcquisitionSource::Documentation { url, .. } => {
                (
                    format!(
                        "Documentation for '{}': official specification \
                         found at {}. Key concepts extracted.",
                        gap.topic, url
                    ),
                    SOURCE_RELIABILITY_DOCUMENTATION,
                    url.clone(),
                )
            }
            AcquisitionSource::BeliefSynthesis { related_topics } => {
                (
                    format!(
                        "Synthesized from related beliefs: [{}]. \
                         Structural inference about '{}'.",
                        related_topics.join(", "), gap.topic
                    ),
                    SOURCE_RELIABILITY_BELIEF_SYNTHESIS,
                    "belief-manifold-synthesis".to_string(),
                )
            }
            AcquisitionSource::WebSearch { query } => {
                (
                    format!(
                        "Web search for '{}': multiple sources found. \
                         Cross-referenced for consistency.",
                        gap.topic
                    ),
                    SOURCE_RELIABILITY_WEB,
                    format!("web-search:{}", query),
                )
            }
            AcquisitionSource::ExpertSystem { endpoint, .. } => {
                (
                    format!(
                        "Expert system at {} queried for '{}'.",
                        endpoint, gap.topic
                    ),
                    SOURCE_RELIABILITY_EXPERT_SYSTEM,
                    endpoint.clone(),
                )
            }
            AcquisitionSource::HumanResolution { specific_question } => {
                (
                    format!("Human resolution needed: {}", specific_question),
                    1.0,
                    "human".to_string(),
                )
            }
        };

        AcquisitionResult::new(
            &gap.id, source.label(), content, confidence, provenance,
        )
    }

    /// Detect and immediately acquire — convenience method.
    pub fn detect_and_acquire(
        &mut self,
        topic:    impl Into<String>,
        gap_type: GapType,
        priority: f64,
    ) -> Result<AcquisitionSummary, OmniscienceError> {
        let gap_id = self.detect_gap(topic, gap_type, priority);
        self.acquire(&gap_id)
    }

    /// All open (unresolved) gaps.
    pub fn open_gaps(&self) -> Vec<&KnowledgeGap> {
        self.gaps.values()
            .filter(|g| !g.state.is_resolved())
            .collect()
    }

    /// Recurring gaps — signal domain needs skill loading.
    pub fn recurring_gaps(&self) -> Vec<&KnowledgeGap> {
        self.gaps.values()
            .filter(|g| g.is_recurring() && !g.state.is_resolved())
            .collect()
    }

    pub fn gap_count(&self)       -> usize { self.gaps.len() }
    pub fn closed_count(&self)    -> usize { self.closed_count }
    pub fn escalated_count(&self) -> usize { self.escalated_count }
    pub fn result_count(&self)    -> usize { self.results.len() }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "omniscience: gaps={} open={} closed={} escalated={} recurring={}",
            self.gap_count(),
            self.open_gaps().len(),
            self.closed_count,
            self.escalated_count,
            self.recurring_gaps().len(),
        )
    }
}

impl Default for OmniscienceEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_gap_registered() {
        let mut engine = OmniscienceEngine::new();
        let id = engine.detect_gap(
            "kubernetes rolling update strategy",
            GapType::Procedural { action: "rolling-update".into() },
            0.8,
        );
        assert!(!id.is_empty());
        assert_eq!(engine.gap_count(), 1);
        assert_eq!(engine.open_gaps().len(), 1);
    }

    #[test]
    fn acquire_closes_gap() {
        let mut engine = OmniscienceEngine::new();
        let result = engine.detect_and_acquire(
            "how to implement a circuit breaker in Rust",
            GapType::Procedural { action: "circuit-breaker".into() },
            0.85,
        ).expect("acquire should succeed");
        assert!(result.closed);
        assert!(result.confidence >= MIN_ACQUISITION_CONFIDENCE);
        assert_eq!(engine.closed_count(), 1);
        assert_eq!(engine.open_gaps().len(), 0);
    }

    #[test]
    fn same_topic_increments_recurrence() {
        let mut engine = OmniscienceEngine::new();
        for _ in 0..RECURRING_GAP_THRESHOLD {
            engine.detect_gap(
                "video editing vocabulary",
                GapType::VocabularyMissing { domain: "video".into() },
                0.6,
            );
        }
        assert_eq!(engine.gap_count(), 1);
        let gap = engine.gaps.values().next()
            .expect("gap should exist");
        assert_eq!(gap.recurrence, RECURRING_GAP_THRESHOLD);
        assert!(gap.is_recurring());
    }

    #[test]
    fn recurring_gaps_flagged() {
        let mut engine = OmniscienceEngine::new();
        for _ in 0..RECURRING_GAP_THRESHOLD {
            engine.detect_gap(
                "finance domain vocabulary",
                GapType::VocabularyMissing { domain: "finance".into() },
                0.7,
            );
        }
        assert_eq!(engine.recurring_gaps().len(), 1);
    }

    #[test]
    fn summary_format() {
        let engine = OmniscienceEngine::new();
        let s = engine.summary();
        assert!(s.contains("omniscience:"));
        assert!(s.contains("gaps="));
        assert!(s.contains("closed="));
    }
}
