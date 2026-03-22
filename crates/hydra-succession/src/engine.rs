//! SuccessionEngine — the entity continuation coordinator.
//! Export -> seal -> verify -> import.
//! The entity survives the substrate change.

use crate::{
    errors::SuccessionError,
    exporter::{InstanceState, SuccessionExporter},
    package::SuccessionPackage,
    verifier::SuccessionVerifier,
};

/// The result of a successful succession import.
#[derive(Debug)]
pub struct SuccessionResult {
    pub package_id: String,
    pub lineage_id: String,
    pub wisdom_days: u32,
    pub soul_entries: usize,
    pub genome_entries: usize,
    pub calibration_profiles: usize,
    pub notes: Vec<String>,
}

impl SuccessionResult {
    pub fn summary(&self) -> String {
        format!(
            "Succession complete: {} days of wisdom, {} soul entries, \
             {} genome entries, {} calibration profiles transferred.",
            self.wisdom_days, self.soul_entries, self.genome_entries, self.calibration_profiles,
        )
    }
}

/// The succession engine.
pub struct SuccessionEngine {
    exporter: SuccessionExporter,
    verifier: SuccessionVerifier,
    current_package: Option<SuccessionPackage>,
    has_imported: bool,
}

impl SuccessionEngine {
    pub fn new() -> Self {
        Self {
            exporter: SuccessionExporter::new(),
            verifier: SuccessionVerifier::new(),
            current_package: None,
            has_imported: false,
        }
    }

    /// Export the current instance's wisdom into a sealed package.
    pub fn export(&mut self, state: &InstanceState) -> Result<&SuccessionPackage, SuccessionError> {
        let package = self.exporter.export(state)?;
        self.current_package = Some(package);
        Ok(self.current_package.as_ref().expect("just set"))
    }

    /// Verify a received succession package.
    pub fn verify(
        &mut self,
        package: &mut SuccessionPackage,
        lineage_id: &str,
    ) -> Result<Vec<String>, SuccessionError> {
        let result = self.verifier.verify(package, lineage_id)?;
        if result.all_pass() {
            package.mark_verified();
            Ok(result.notes)
        } else {
            package.reject("Verification failed");
            Err(SuccessionError::IntegrityFailure)
        }
    }

    /// Import a verified succession package into this instance.
    /// One-time operation per instance.
    pub fn import(
        &mut self,
        package: &mut SuccessionPackage,
        lineage_id: &str,
    ) -> Result<SuccessionResult, SuccessionError> {
        if self.has_imported {
            return Err(SuccessionError::AlreadyImported);
        }

        // Verify if not already verified
        if package.state.label() != "verified" {
            self.verify(package, lineage_id)?;
        }

        let result = SuccessionResult {
            package_id: package.id.clone(),
            lineage_id: package.lineage_id.clone(),
            wisdom_days: package.wisdom_days,
            soul_entries: package.soul_entry_count(),
            genome_entries: package.genome_entry_count(),
            calibration_profiles: package.calibration_profile_count(),
            notes: vec![
                format!(
                    "Soul: {} entries ({} days of orientation) integrated.",
                    package.soul_entry_count(),
                    package.wisdom_days
                ),
                format!(
                    "Genome: {} proven approaches across {} domains integrated.",
                    package.genome_entry_count(),
                    package.genome.total_domains
                ),
                format!(
                    "Calibration: {} domain bias profiles integrated.",
                    package.calibration_profile_count()
                ),
                format!(
                    "Morphic signature: {} days deep. Lineage '{}' continuous.",
                    package.morphic.days_depth, package.lineage_id
                ),
            ],
        };

        package.mark_imported();
        self.has_imported = true;
        Ok(result)
    }

    /// Full succession: export -> seal -> verify -> import.
    /// The complete entity continuation protocol.
    pub fn full_succession(
        &mut self,
        from_state: &InstanceState,
    ) -> Result<SuccessionResult, SuccessionError> {
        if self.has_imported {
            return Err(SuccessionError::AlreadyImported);
        }

        let mut package = self.exporter.export(from_state)?;

        let lineage_id = from_state.lineage_id.clone();
        let ver_result = self.verifier.verify(&package, &lineage_id)?;
        if !ver_result.all_pass() {
            return Err(SuccessionError::IntegrityFailure);
        }
        package.mark_verified();

        let result = SuccessionResult {
            package_id: package.id.clone(),
            lineage_id: package.lineage_id.clone(),
            wisdom_days: package.wisdom_days,
            soul_entries: package.soul_entry_count(),
            genome_entries: package.genome_entry_count(),
            calibration_profiles: package.calibration_profile_count(),
            notes: ver_result.notes,
        };

        package.mark_imported();
        self.has_imported = true;
        Ok(result)
    }

    pub fn has_imported(&self) -> bool {
        self.has_imported
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "succession: exported={} imported={}",
            self.current_package.is_some(),
            self.has_imported,
        )
    }
}

impl Default for SuccessionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v1_state() -> InstanceState {
        InstanceState {
            instance_id: "hydra-v1".into(),
            lineage_id: "hydra-agentra-lineage".into(),
            days_running: 7300,
            soul_entries: 500,
            genome_entries: 2400,
            calibration_profiles: 47,
        }
    }

    #[test]
    fn full_succession_transfers_wisdom() {
        let mut engine = SuccessionEngine::new();
        let result = engine.full_succession(&v1_state()).expect("should succeed");
        assert_eq!(result.wisdom_days, 7300);
        assert_eq!(result.soul_entries, 500);
        assert_eq!(result.genome_entries, 2400);
        assert_eq!(result.calibration_profiles, 47);
        assert!(engine.has_imported());
    }

    #[test]
    fn double_import_rejected() {
        let mut engine = SuccessionEngine::new();
        engine
            .full_succession(&v1_state())
            .expect("first should succeed");
        let second = engine.full_succession(&v1_state());
        assert!(matches!(second, Err(SuccessionError::AlreadyImported)));
    }

    #[test]
    fn summary_format() {
        let engine = SuccessionEngine::new();
        let s = engine.summary();
        assert!(s.contains("succession:"));
        assert!(s.contains("exported="));
        assert!(s.contains("imported="));
    }
}
