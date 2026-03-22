//! CrystallizerEngine — the artifact generation coordinator.

use crate::{
    artifact::{ArtifactKind, CrystallizedArtifact},
    constants::MAX_STORED_ARTIFACTS,
    errors::CrystallizerError,
    generator::ArtifactGenerator,
    source::CrystallizationSource,
};

/// The crystallizer engine.
pub struct CrystallizerEngine {
    artifacts: Vec<CrystallizedArtifact>,
    generator: ArtifactGenerator,
}

impl CrystallizerEngine {
    pub fn new() -> Self {
        Self {
            artifacts: Vec::new(),
            generator: ArtifactGenerator::new(),
        }
    }

    /// Crystallize a playbook from a source.
    pub fn crystallize_playbook(
        &mut self,
        source: &CrystallizationSource,
    ) -> Result<&CrystallizedArtifact, CrystallizerError> {
        let artifact = self.generator.generate_playbook(source)?;
        self.store(artifact)
    }

    /// Crystallize a post-mortem from a source.
    pub fn crystallize_postmortem(
        &mut self,
        source: &CrystallizationSource,
        incident_description: &str,
    ) -> Result<&CrystallizedArtifact, CrystallizerError> {
        let artifact = self
            .generator
            .generate_postmortem(source, incident_description)?;
        self.store(artifact)
    }

    /// Crystallize a knowledge base from a source.
    pub fn crystallize_knowledge_base(
        &mut self,
        source: &CrystallizationSource,
    ) -> Result<&CrystallizedArtifact, CrystallizerError> {
        let artifact = self.generator.generate_knowledge_base(source)?;
        self.store(artifact)
    }

    fn store(
        &mut self,
        artifact: CrystallizedArtifact,
    ) -> Result<&CrystallizedArtifact, CrystallizerError> {
        if self.artifacts.len() >= MAX_STORED_ARTIFACTS {
            return Err(CrystallizerError::StoreFull {
                max: MAX_STORED_ARTIFACTS,
            });
        }
        self.artifacts.push(artifact);
        // Safe: we just pushed an element
        Ok(self.artifacts.last().expect("just pushed"))
    }

    /// All artifacts for a domain.
    pub fn artifacts_for_domain(&self, domain: &str) -> Vec<&CrystallizedArtifact> {
        self.artifacts
            .iter()
            .filter(|a| a.domain == domain)
            .collect()
    }

    pub fn artifact_count(&self) -> usize {
        self.artifacts.len()
    }

    /// Summary for TUI / intelligence brief.
    pub fn summary(&self) -> String {
        let playbooks = self
            .artifacts
            .iter()
            .filter(|a| a.kind == ArtifactKind::Playbook)
            .count();
        let postmortems = self
            .artifacts
            .iter()
            .filter(|a| a.kind == ArtifactKind::PostMortem)
            .count();
        format!(
            "crystallizer: artifacts={} playbooks={} post-mortems={}",
            self.artifact_count(),
            playbooks,
            postmortems,
        )
    }
}

impl Default for CrystallizerEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::CrystallizationSource;
    use hydra_settlement::{CostClass, CostItem, Outcome, SettlementRecord};

    fn success_record(id: &str, domain: &str) -> SettlementRecord {
        let costs = vec![CostItem::new(CostClass::DirectExecution, 2000, 5.0, 1000)];
        SettlementRecord::new(
            id,
            "deploy",
            domain,
            "deploy service",
            Outcome::Success {
                description: "done".into(),
            },
            costs,
            1000,
            1,
        )
    }

    fn build_source(domain: &str, n: usize) -> CrystallizationSource {
        let mut src = CrystallizationSource::new(domain)
            .with_approach("rotate credentials first", 0.87)
            .with_avoidable("concurrent lock");
        for i in 0..n {
            src = src.with_success(success_record(&format!("s{}", i), domain));
        }
        src
    }

    #[test]
    fn crystallize_playbook() {
        let mut engine = CrystallizerEngine::new();
        let src = build_source("engineering", 6);
        engine.crystallize_playbook(&src).expect("playbook");
        assert_eq!(engine.artifact_count(), 1);
    }

    #[test]
    fn domain_filter() {
        let mut engine = CrystallizerEngine::new();
        engine
            .crystallize_playbook(&build_source("engineering", 6))
            .expect("eng playbook");
        engine
            .crystallize_knowledge_base(&build_source("fintech", 3))
            .expect("fintech kb");
        assert_eq!(engine.artifacts_for_domain("engineering").len(), 1);
        assert_eq!(engine.artifacts_for_domain("fintech").len(), 1);
    }

    #[test]
    fn summary_format() {
        let engine = CrystallizerEngine::new();
        let s = engine.summary();
        assert!(s.contains("crystallizer:"));
    }
}
