//! SuccessionExporter — produces a sealed succession package
//! from the current instance's accumulated state.

use crate::{errors::SuccessionError, package::SuccessionPackage, payload::*};

/// Simulated state of a Hydra instance for export.
/// In production: reads from hydra-soul, hydra-genome,
/// hydra-calibration, hydra-morphic directly.
#[derive(Debug, Clone)]
pub struct InstanceState {
    pub instance_id: String,
    pub lineage_id: String,
    pub days_running: u32,
    pub soul_entries: usize,
    pub genome_entries: usize,
    pub calibration_profiles: usize,
}

/// The succession exporter.
pub struct SuccessionExporter;

impl SuccessionExporter {
    pub fn new() -> Self {
        Self
    }

    /// Export the current instance state into a succession package.
    pub fn export(&self, state: &InstanceState) -> Result<SuccessionPackage, SuccessionError> {
        if state.soul_entries < crate::constants::MIN_SOUL_ENTRIES_FOR_SUCCESSION {
            return Err(SuccessionError::InsufficientSoulData {
                count: state.soul_entries,
                min: crate::constants::MIN_SOUL_ENTRIES_FOR_SUCCESSION,
            });
        }
        if state.genome_entries < crate::constants::MIN_GENOME_ENTRIES_FOR_SUCCESSION {
            return Err(SuccessionError::InsufficientGenomeData {
                count: state.genome_entries,
                min: crate::constants::MIN_GENOME_ENTRIES_FOR_SUCCESSION,
            });
        }

        let soul = SoulPayload::simulated(state.days_running, state.soul_entries);
        let genome = GenomePayload::simulated(state.genome_entries);
        let calibration = CalibrationPayload::simulated(state.calibration_profiles);
        let morphic = MorphicPayload::simulated(state.days_running, &state.lineage_id);

        let package = SuccessionPackage::seal(
            &state.instance_id,
            &state.lineage_id,
            soul,
            genome,
            calibration,
            morphic,
        );

        Ok(package)
    }
}

impl Default for SuccessionExporter {
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
            genome_entries: 2_400,
            calibration_profiles: 47,
        }
    }

    #[test]
    fn export_succeeds_for_mature_instance() {
        let e = SuccessionExporter::new();
        let pkg = e.export(&v1_state()).expect("should export");
        assert_eq!(pkg.wisdom_days, 7300);
        assert_eq!(pkg.soul_entry_count(), 500);
        assert_eq!(pkg.genome_entry_count(), 2_400);
        assert!(pkg.verify_integrity());
    }

    #[test]
    fn export_fails_for_empty_soul() {
        let e = SuccessionExporter::new();
        let state = InstanceState {
            soul_entries: 0,
            ..v1_state()
        };
        let r = e.export(&state);
        assert!(matches!(
            r,
            Err(SuccessionError::InsufficientSoulData { .. })
        ));
    }
}
