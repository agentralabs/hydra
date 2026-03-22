//! SuccessionVerifier — three-gate verification before any import.
//! All three gates must pass. One failure = rejected package.

use crate::{
    constants::PACKAGE_VALIDITY_DAYS, errors::SuccessionError, package::SuccessionPackage,
};

/// The result of a verification pass.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub integrity_ok: bool,
    pub identity_ok: bool,
    pub constitution_ok: bool,
    pub notes: Vec<String>,
}

impl VerificationResult {
    pub fn all_pass(&self) -> bool {
        self.integrity_ok && self.identity_ok && self.constitution_ok
    }
}

/// The succession verifier — three independent gates.
pub struct SuccessionVerifier;

impl SuccessionVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Verify a succession package before import.
    /// All three gates must pass.
    pub fn verify(
        &self,
        package: &SuccessionPackage,
        lineage_id: &str,
    ) -> Result<VerificationResult, SuccessionError> {
        let mut result = VerificationResult {
            integrity_ok: false,
            identity_ok: false,
            constitution_ok: false,
            notes: Vec::new(),
        };

        // -- GATE 1: INTEGRITY --
        if !package.verify_integrity() {
            return Err(SuccessionError::IntegrityFailure);
        }
        if package.is_expired() {
            let days_ago = (chrono::Utc::now() - package.sealed_at).num_days();
            return Err(SuccessionError::PackageExpired {
                issued_days_ago: days_ago,
                max: PACKAGE_VALIDITY_DAYS,
            });
        }
        result.integrity_ok = true;
        result.notes.push(format!(
            "Integrity: SHA256 verified. Package age: {} days. Wisdom: {} days.",
            (chrono::Utc::now() - package.sealed_at).num_days(),
            package.wisdom_days,
        ));

        // -- GATE 2: IDENTITY --
        if package.lineage_id != lineage_id {
            return Err(SuccessionError::IdentityMismatch);
        }
        if !package.morphic.verify() {
            return Err(SuccessionError::IdentityMismatch);
        }
        result.identity_ok = true;
        result.notes.push(format!(
            "Identity: lineage '{}' confirmed. Morphic signature depth: {} days.",
            lineage_id, package.morphic.days_depth,
        ));

        // -- GATE 3: CONSTITUTION --
        if package.soul_entry_count() < crate::constants::MIN_SOUL_ENTRIES_FOR_SUCCESSION {
            return Err(SuccessionError::ConstitutionalViolation {
                law: "soul_non_erasable: package contains no soul entries".into(),
            });
        }
        if package.genome_entry_count() < crate::constants::MIN_GENOME_ENTRIES_FOR_SUCCESSION {
            return Err(SuccessionError::ConstitutionalViolation {
                law: "genome_permanent: package contains no genome entries".into(),
            });
        }
        result.constitution_ok = true;
        result.notes.push(format!(
            "Constitution: {} soul entries, {} genome entries, {} calibration profiles. All laws satisfied.",
            package.soul_entry_count(),
            package.genome_entry_count(),
            package.calibration_profile_count(),
        ));

        Ok(result)
    }
}

impl Default for SuccessionVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::SuccessionPackage;
    use crate::payload::*;

    fn make_valid_package() -> SuccessionPackage {
        SuccessionPackage::seal(
            "hydra-v1",
            "hydra-agentra-lineage",
            SoulPayload::simulated(7300, 50),
            GenomePayload::simulated(100),
            CalibrationPayload::simulated(5),
            MorphicPayload::simulated(7300, "hydra-agentra-lineage"),
        )
    }

    #[test]
    fn valid_package_passes_all_gates() {
        let pkg = make_valid_package();
        let v = SuccessionVerifier::new();
        let result = v
            .verify(&pkg, "hydra-agentra-lineage")
            .expect("should verify");
        assert!(result.all_pass());
        assert_eq!(result.notes.len(), 3);
    }

    #[test]
    fn wrong_lineage_fails_identity_gate() {
        let pkg = make_valid_package();
        let v = SuccessionVerifier::new();
        let r = v.verify(&pkg, "different-lineage");
        assert!(matches!(r, Err(SuccessionError::IdentityMismatch)));
    }

    #[test]
    fn empty_soul_fails_constitution() {
        let mut pkg = make_valid_package();
        pkg.soul = SoulPayload {
            entries: vec![],
            days_accumulated: 0,
            founding_statement: "".into(),
        };
        let v = SuccessionVerifier::new();
        let r = v.verify(&pkg, "hydra-agentra-lineage");
        assert!(matches!(
            r,
            Err(SuccessionError::ConstitutionalViolation { .. })
        ));
    }

    #[test]
    fn empty_genome_fails_constitution() {
        let mut pkg = make_valid_package();
        pkg.genome = GenomePayload {
            entries: vec![],
            total_domains: 0,
        };
        let v = SuccessionVerifier::new();
        let r = v.verify(&pkg, "hydra-agentra-lineage");
        assert!(matches!(
            r,
            Err(SuccessionError::ConstitutionalViolation { .. })
        ));
    }
}
