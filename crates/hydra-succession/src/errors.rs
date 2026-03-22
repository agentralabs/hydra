use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum SuccessionError {
    #[error("Package integrity check failed — hash mismatch")]
    IntegrityFailure,

    #[error("Morphic signature mismatch — package is from a different entity lineage")]
    IdentityMismatch,

    #[error("Package expired — issued {issued_days_ago} days ago (max {max} days)")]
    PackageExpired { issued_days_ago: i64, max: i64 },

    #[error("Insufficient soul data: {count} entries (need {min})")]
    InsufficientSoulData { count: usize, min: usize },

    #[error("Insufficient genome data: {count} entries (need {min})")]
    InsufficientGenomeData { count: usize, min: usize },

    #[error("Constitutional violation in succession package: {law}")]
    ConstitutionalViolation { law: String },

    #[error("Import already applied — succession is one-time per instance")]
    AlreadyImported,
}

impl SuccessionError {
    pub fn is_hard_stop(&self) -> bool {
        matches!(
            self,
            Self::IntegrityFailure | Self::IdentityMismatch | Self::ConstitutionalViolation { .. }
        )
    }
}
