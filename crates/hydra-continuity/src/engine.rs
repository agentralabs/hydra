//! ContinuityEngine — the entity arc coordinator.

use crate::{
    arc::EntityArc,
    errors::ContinuityError,
};
use hydra_succession::SuccessionPackage;
use std::collections::HashMap;

/// The continuity engine.
pub struct ContinuityEngine {
    arcs: HashMap<String, EntityArc>,
}

impl ContinuityEngine {
    pub fn new() -> Self {
        Self {
            arcs: HashMap::new(),
        }
    }

    /// Build or update an arc from a succession package.
    pub fn record_from_succession(
        &mut self,
        package: &SuccessionPackage,
    ) -> &EntityArc {
        let arc = self
            .arcs
            .entry(package.lineage_id.clone())
            .or_insert_with(|| EntityArc::new(&package.lineage_id));

        arc.from_succession(
            &package.lineage_id,
            package.wisdom_days,
            package.soul_entry_count(),
            package.genome_entry_count(),
            &package.morphic.signature_chain,
        );
        arc
    }

    /// Get the arc for a lineage.
    pub fn arc(&self, lineage_id: &str) -> Option<&EntityArc> {
        self.arcs.get(lineage_id)
    }

    /// Prove lineage continuity.
    pub fn prove_lineage(
        &self,
        lineage_id: &str,
    ) -> Result<bool, ContinuityError> {
        let arc = self.arcs.get(lineage_id).ok_or_else(|| {
            ContinuityError::ArcNotFound {
                lineage_id: lineage_id.to_string(),
            }
        })?;
        Ok(arc.prove_lineage(lineage_id))
    }

    /// Prove that v2 is the same entity as v1 (succession proof).
    pub fn prove_succession(
        &self,
        v1_lineage: &str,
        v2_lineage: &str,
    ) -> Result<bool, ContinuityError> {
        let v1 = self.arcs.get(v1_lineage).ok_or_else(|| {
            ContinuityError::ArcNotFound {
                lineage_id: v1_lineage.to_string(),
            }
        })?;
        let v2 = self.arcs.get(v2_lineage).ok_or_else(|| {
            ContinuityError::ArcNotFound {
                lineage_id: v2_lineage.to_string(),
            }
        })?;
        Ok(v2.proves_succession_from(v1))
    }

    pub fn lineage_count(&self) -> usize {
        self.arcs.len()
    }

    pub fn total_checkpoint_count(&self) -> usize {
        self.arcs.values().map(|a| a.checkpoint_count()).sum()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "continuity: lineages={} checkpoints={}",
            self.lineage_count(),
            self.total_checkpoint_count(),
        )
    }
}

impl Default for ContinuityEngine {
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
                lineage_id: "hydra-agentra".into(),
                days_running: 7300,
                soul_entries: 730,
                genome_entries: 14600,
                calibration_profiles: 47,
            })
            .expect("should export mature package")
    }

    #[test]
    fn arc_from_succession() {
        let mut engine = ContinuityEngine::new();
        let pkg = mature_package();
        engine.record_from_succession(&pkg);
        assert_eq!(engine.lineage_count(), 1);
        assert!(engine.total_checkpoint_count() > 0);
    }

    #[test]
    fn lineage_proof() {
        let mut engine = ContinuityEngine::new();
        let pkg = mature_package();
        engine.record_from_succession(&pkg);
        assert!(engine.prove_lineage("hydra-agentra").expect("should prove"));
    }

    #[test]
    fn summary_format() {
        let engine = ContinuityEngine::new();
        let s = engine.summary();
        assert!(s.contains("continuity:"));
    }
}
