//! InfluenceEngine — THE FINAL COORDINATOR.
//! THE LAST CRATE. LAYER 7 CLOSES HERE.

use crate::{
    adoption::AdoptionRecord,
    constants::*,
    discovery::{DiscoveryQuery, DiscoveryResult, discover},
    errors::InfluenceError,
    publication::{PatternCategory, PublishedPattern},
};
use std::collections::HashMap;

/// The influence engine — final coordinator of Hydra.
pub struct InfluenceEngine {
    registry:  Vec<PublishedPattern>,
    adoptions: Vec<AdoptionRecord>,
    /// Map of pattern_id -> Vec<adopting_lineage>
    adoption_index: HashMap<String, Vec<String>>,
}

impl InfluenceEngine {
    pub fn new() -> Self {
        Self {
            registry:       Vec::new(),
            adoptions:      Vec::new(),
            adoption_index: HashMap::new(),
        }
    }

    /// Publish a pattern to the influence network.
    #[allow(clippy::too_many_arguments)]
    pub fn publish(
        &mut self,
        source_lineage:  &str,
        title:           impl Into<String>,
        description:     impl Into<String>,
        category:        PatternCategory,
        domain_tags:     Vec<String>,
        pattern_content: impl Into<String>,
        evidence_count:  usize,
        confidence:      f64,
        source_days:     u32,
    ) -> Result<String, InfluenceError> {
        if evidence_count < MIN_EVIDENCE_FOR_PUBLICATION {
            return Err(InfluenceError::InsufficientEvidence {
                count: evidence_count,
                min:   MIN_EVIDENCE_FOR_PUBLICATION,
            });
        }
        if confidence < MIN_CONFIDENCE_FOR_PUBLICATION {
            return Err(InfluenceError::LowConfidence {
                confidence,
                min: MIN_CONFIDENCE_FOR_PUBLICATION,
            });
        }
        if self.registry.len() >= MAX_PUBLISHED_PATTERNS {
            return Err(InfluenceError::RegistryFull {
                max: MAX_PUBLISHED_PATTERNS,
            });
        }

        let pattern = PublishedPattern::new(
            source_lineage, title, description, category,
            domain_tags, pattern_content, evidence_count, confidence, source_days,
        );
        let id = pattern.id.clone();
        self.registry.push(pattern);
        Ok(id)
    }

    /// Discover patterns relevant to a context.
    pub fn discover(
        &self,
        query: &DiscoveryQuery,
    ) -> Vec<DiscoveryResult> {
        discover(&self.registry, query)
    }

    /// Adopt a pattern from the network.
    pub fn adopt(
        &mut self,
        pattern_id:       &str,
        adopting_lineage: &str,
    ) -> Result<&AdoptionRecord, InfluenceError> {
        let pattern = self.registry.iter()
            .find(|p| p.id == pattern_id)
            .ok_or_else(|| InfluenceError::PatternNotFound {
                id: pattern_id.to_string(),
            })?;

        // Check not already adopted
        if self.adoption_index.get(pattern_id)
            .map(|v| v.contains(&adopting_lineage.to_string()))
            .unwrap_or(false)
        {
            return Err(InfluenceError::AlreadyAdopted {
                id:      pattern_id.to_string(),
                lineage: adopting_lineage.to_string(),
            });
        }

        let record = AdoptionRecord::new(
            pattern_id, &pattern.title,
            adopting_lineage, &pattern.source_lineage,
            pattern.confidence, pattern.source_days,
        );

        if self.adoptions.len() >= MAX_ADOPTION_RECORDS {
            self.adoptions.remove(0);
        }

        self.adoptions.push(record);
        self.adoption_index
            .entry(pattern_id.to_string())
            .or_default()
            .push(adopting_lineage.to_string());

        // Update pattern adoption count
        if let Some(p) = self.registry.iter_mut().find(|p| p.id == pattern_id) {
            p.adoption_count += 1;
        }

        Ok(self.adoptions.last().expect("just pushed"))
    }

    /// Record an outcome for an adopted pattern — feeds back to source.
    pub fn record_outcome(
        &mut self,
        pattern_id:       &str,
        adopting_lineage: &str,
        success:          bool,
    ) {
        // Update adoption record
        if let Some(record) = self.adoptions.iter_mut()
            .find(|a| a.pattern_id == pattern_id
                && a.adopting_lineage == adopting_lineage)
        {
            record.record_outcome(success);
        }
        // Feed back to published pattern
        if let Some(p) = self.registry.iter_mut()
            .find(|p| p.id == pattern_id)
        {
            p.record_outcome(success);
        }
    }

    pub fn published_count(&self)    -> usize { self.registry.len() }
    pub fn adoption_count(&self)     -> usize { self.adoptions.len() }

    pub fn adoption_count_for(&self, pattern_id: &str) -> usize {
        self.adoption_index.get(pattern_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "influence: published={} adoptions={}",
            self.published_count(),
            self.adoption_count(),
        )
    }
}

impl Default for InfluenceEngine { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    fn publish_circuit_breaker(engine: &mut InfluenceEngine) -> String {
        engine.publish(
            "hydra-agentra-lineage",
            "Circuit Breaker at Service Boundaries",
            "Install circuit breakers at all service dependency boundaries",
            PatternCategory::Engineering,
            vec!["engineering".into(), "microservices".into()],
            "Step 1: identify boundaries. Step 2: configure thresholds. Step 3: monitor.",
            47, 0.92, 7300,
        ).expect("should publish")
    }

    #[test]
    fn publish_and_discover() {
        let mut engine = InfluenceEngine::new();
        publish_circuit_breaker(&mut engine);
        let results = engine.discover(&DiscoveryQuery {
            domain: Some("engineering".into()),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn adopt_records_provenance() {
        let mut engine = InfluenceEngine::new();
        let id = publish_circuit_breaker(&mut engine);
        let adoption = engine.adopt(&id, "hydra-partner-lineage").expect("should adopt");
        assert!(adoption.provenance.contains("hydra-agentra-lineage"));
        assert_eq!(engine.adoption_count_for(&id), 1);
    }

    #[test]
    fn double_adoption_rejected() {
        let mut engine = InfluenceEngine::new();
        let id = publish_circuit_breaker(&mut engine);
        engine.adopt(&id, "hydra-partner").expect("first adopt");
        let r = engine.adopt(&id, "hydra-partner");
        assert!(matches!(r, Err(InfluenceError::AlreadyAdopted { .. })));
    }

    #[test]
    fn outcome_feedback_raises_confidence() {
        let mut engine = InfluenceEngine::new();
        let id = publish_circuit_breaker(&mut engine);
        let before = engine.registry[0].confidence;
        engine.adopt(&id, "hydra-partner").expect("should adopt");
        engine.record_outcome(&id, "hydra-partner", true);
        assert!(engine.registry[0].confidence > before);
    }

    #[test]
    fn insufficient_evidence_rejected() {
        let mut engine = InfluenceEngine::new();
        let r = engine.publish(
            "lineage", "title", "desc",
            PatternCategory::Engineering, vec![], "content",
            2, 0.85, 1000,
        );
        assert!(matches!(r, Err(InfluenceError::InsufficientEvidence { .. })));
    }

    #[test]
    fn summary_format() {
        let engine = InfluenceEngine::new();
        let s = engine.summary();
        assert!(s.contains("influence:"));
    }
}
