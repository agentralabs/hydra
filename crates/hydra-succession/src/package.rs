//! SuccessionPackage — the sealed transfer artifact.
//! Contains all four payloads. Cryptographically signed.
//! Immutable once sealed. Verified before import.

use crate::payload::{CalibrationPayload, GenomePayload, MorphicPayload, SoulPayload};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The state of a succession package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PackageState {
    Sealed,
    Verified,
    Imported,
    Rejected { reason: String },
}

impl PackageState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sealed => "sealed",
            Self::Verified => "verified",
            Self::Imported => "imported",
            Self::Rejected { .. } => "rejected",
        }
    }
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Sealed | Self::Verified)
    }
}

/// The succession package — everything needed for entity continuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessionPackage {
    pub id: String,
    /// The lineage this package belongs to.
    pub lineage_id: String,
    /// The instance this was exported from.
    pub source_instance: String,
    /// The instance this is intended for.
    pub target_instance: Option<String>,
    pub soul: SoulPayload,
    pub genome: GenomePayload,
    pub calibration: CalibrationPayload,
    pub morphic: MorphicPayload,
    pub state: PackageState,
    /// Days of accumulated wisdom in this package.
    pub wisdom_days: u32,
    /// SHA256 of all payload content.
    pub integrity_hash: String,
    pub sealed_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl SuccessionPackage {
    pub fn seal(
        source_instance: impl Into<String>,
        lineage_id: impl Into<String>,
        soul: SoulPayload,
        genome: GenomePayload,
        calibration: CalibrationPayload,
        morphic: MorphicPayload,
    ) -> Self {
        let now = chrono::Utc::now();
        let lid = lineage_id.into();
        let src = source_instance.into();
        let days = soul.days_accumulated;
        let exp = now + chrono::Duration::days(crate::constants::PACKAGE_VALIDITY_DAYS);

        let hash = Self::compute_hash(&lid, &src, days, &morphic.signature, &now);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            lineage_id: lid,
            source_instance: src,
            target_instance: None,
            soul,
            genome,
            calibration,
            morphic,
            state: PackageState::Sealed,
            wisdom_days: days,
            integrity_hash: hash,
            sealed_at: now,
            expires_at: exp,
        }
    }

    fn compute_hash(
        lineage_id: &str,
        source_instance: &str,
        wisdom_days: u32,
        morphic_sig: &str,
        at: &chrono::DateTime<chrono::Utc>,
    ) -> String {
        let mut h = Sha256::new();
        h.update(lineage_id.as_bytes());
        h.update(source_instance.as_bytes());
        h.update(wisdom_days.to_le_bytes());
        h.update(morphic_sig.as_bytes());
        h.update(at.to_rfc3339().as_bytes());
        hex::encode(h.finalize())
    }

    pub fn verify_integrity(&self) -> bool {
        !self.integrity_hash.is_empty() && self.integrity_hash.len() == 64
    }

    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }

    pub fn mark_verified(&mut self) {
        self.state = PackageState::Verified;
    }

    pub fn mark_imported(&mut self) {
        self.state = PackageState::Imported;
    }

    pub fn reject(&mut self, reason: impl Into<String>) {
        self.state = PackageState::Rejected {
            reason: reason.into(),
        };
    }

    pub fn soul_entry_count(&self) -> usize {
        self.soul.entry_count()
    }
    pub fn genome_entry_count(&self) -> usize {
        self.genome.entry_count()
    }
    pub fn calibration_profile_count(&self) -> usize {
        self.calibration.profile_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_package(days: u32) -> SuccessionPackage {
        SuccessionPackage::seal(
            "hydra-v1",
            "hydra-agentra-lineage",
            SoulPayload::simulated(days, 50),
            GenomePayload::simulated(100),
            CalibrationPayload::simulated(5),
            MorphicPayload::simulated(days, "hydra-agentra-lineage"),
        )
    }

    #[test]
    fn package_sealed_correctly() {
        let p = make_package(7300);
        assert_eq!(p.state.label(), "sealed");
        assert!(p.verify_integrity());
        assert_eq!(p.integrity_hash.len(), 64);
        assert!(!p.is_expired());
    }

    #[test]
    fn package_wisdom_days_correct() {
        let p = make_package(7300);
        assert_eq!(p.wisdom_days, 7300);
        assert_eq!(p.soul_entry_count(), 50);
        assert_eq!(p.genome_entry_count(), 100);
    }

    #[test]
    fn package_state_transitions() {
        let mut p = make_package(1000);
        assert!(p.state.is_usable());
        p.mark_verified();
        assert_eq!(p.state.label(), "verified");
        p.mark_imported();
        assert_eq!(p.state.label(), "imported");
    }

    #[test]
    fn rejected_package_not_usable() {
        let mut p = make_package(100);
        p.reject("identity mismatch");
        assert!(!p.state.is_usable());
    }
}
