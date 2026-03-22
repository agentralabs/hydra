//! LegacyEngine — the permanent knowledge archive coordinator.

use crate::{
    artifact::{LegacyArtifact, LegacyKind},
    builder::LegacyBuilder,
    constants::MAX_LEGACY_ARTIFACTS,
    errors::LegacyError,
};
use hydra_succession::SuccessionPackage;

/// The legacy engine.
pub struct LegacyEngine {
    archive: Vec<LegacyArtifact>,
    builder: LegacyBuilder,
}

impl LegacyEngine {
    pub fn new() -> Self {
        Self {
            archive: Vec::new(),
            builder: LegacyBuilder::new(),
        }
    }

    /// Publish a knowledge record to the permanent archive.
    pub fn publish_knowledge(
        &mut self,
        package: &SuccessionPackage,
        domain: &str,
    ) -> Result<&LegacyArtifact, LegacyError> {
        let artifact = self.builder.knowledge_record(package, domain)?;
        self.store(artifact)
    }

    /// Publish an operational record.
    pub fn publish_operational(
        &mut self,
        package: &SuccessionPackage,
        period: &str,
    ) -> Result<&LegacyArtifact, LegacyError> {
        let artifact = self.builder.operational_record(package, period)?;
        self.store(artifact)
    }

    /// Publish a wisdom record.
    pub fn publish_wisdom(
        &mut self,
        package: &SuccessionPackage,
        domain: &str,
    ) -> Result<&LegacyArtifact, LegacyError> {
        let artifact = self.builder.wisdom_record(package, domain)?;
        self.store(artifact)
    }

    fn store(
        &mut self,
        artifact: LegacyArtifact,
    ) -> Result<&LegacyArtifact, LegacyError> {
        if self.archive.len() >= MAX_LEGACY_ARTIFACTS {
            return Err(LegacyError::ArchiveFull {
                max: MAX_LEGACY_ARTIFACTS,
            });
        }
        if !artifact.verify_integrity() {
            return Err(LegacyError::IntegrityFailure {
                id: artifact.id.clone(),
            });
        }
        self.archive.push(artifact);
        // Safe: we just pushed, so last() is always Some
        Ok(self.archive.last().expect("just pushed an artifact"))
    }

    pub fn artifacts_for_lineage(&self, lineage_id: &str) -> Vec<&LegacyArtifact> {
        self.archive
            .iter()
            .filter(|a| a.lineage_id == lineage_id)
            .collect()
    }

    pub fn artifact_count(&self) -> usize {
        self.archive.len()
    }

    pub fn knowledge_count(&self) -> usize {
        self.archive
            .iter()
            .filter(|a| matches!(a.kind, LegacyKind::KnowledgeRecord { .. }))
            .count()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        let mut lineages: Vec<_> = self
            .archive
            .iter()
            .map(|a| a.lineage_id.as_str())
            .collect();
        lineages.sort();
        lineages.dedup();

        format!(
            "legacy: artifacts={} knowledge={} lineages={}",
            self.artifact_count(),
            self.knowledge_count(),
            lineages.len(),
        )
    }
}

impl Default for LegacyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_succession::{InstanceState, SuccessionExporter};

    fn mature_package() -> SuccessionPackage {
        SuccessionExporter::new()
            .export(&InstanceState {
                instance_id: "v1".into(),
                lineage_id: "hydra-agentra-lineage".into(),
                days_running: 7300,
                soul_entries: 500,
                genome_entries: 2400,
                calibration_profiles: 47,
            })
            .expect("should export mature package")
    }

    #[test]
    fn publish_all_three_types() {
        let mut engine = LegacyEngine::new();
        let pkg = mature_package();
        engine
            .publish_knowledge(&pkg, "engineering")
            .expect("knowledge should publish");
        engine
            .publish_operational(&pkg, "20yr history")
            .expect("operational should publish");
        engine
            .publish_wisdom(&pkg, "fintech")
            .expect("wisdom should publish");
        assert_eq!(engine.artifact_count(), 3);
    }

    #[test]
    fn summary_format() {
        let engine = LegacyEngine::new();
        let s = engine.summary();
        assert!(s.contains("legacy:"));
    }
}
