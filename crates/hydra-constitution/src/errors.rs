//! All error types for hydra-constitution.

use crate::laws::LawId;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ConstitutionError {
    /// A constitutional law was violated.
    #[error("Constitutional violation — {law}: {reason}")]
    LawViolation { law: LawId, reason: String },

    /// Attempted to mutate an immutable receipt.
    #[error("Receipt immutability violated: cannot modify receipt '{receipt_id}'")]
    ReceiptMutationAttempt { receipt_id: String },

    /// Attempted to elevate trust tier without authority.
    #[error("Trust escalation attempted: tier {from} → tier {to} (not permitted)")]
    TrustEscalationAttempt { from: u8, to: u8 },

    /// Attempted identity impersonation of a reserved entity.
    #[error("Identity impersonation attempted: '{claimed}' is a reserved identity")]
    IdentityImpersonation { claimed: String },

    /// Attempted memory overwrite without revision provenance.
    #[error("Memory overwrite without provenance: all memory changes require a causal source")]
    MemoryOverwriteWithoutProvenance,

    /// Attempted to access constitution crate from a runtime path.
    #[error("Constitution runtime access attempted from: '{caller}'")]
    ConstitutionRuntimeAccess { caller: String },

    /// Animus internal bus integrity violated.
    #[error("Animus bus integrity violated: {reason}")]
    AnimusBusViolation { reason: String },

    /// Multiple principals detected — only one permitted.
    #[error("Multiple principals detected: {count} found, maximum is 1")]
    MultiplePrincipals { count: usize },

    /// Action has no traceable causal origin.
    #[error("Orphan action: '{action_id}' has no causal chain to constitutional identity")]
    OrphanAction { action_id: String },

    /// Causal chain does not terminate at the constitutional identity.
    #[error("Causal chain incomplete: does not terminate at constitutional identity")]
    CausalChainIncomplete,

    /// Invalid trust tier value.
    #[error("Invalid trust tier: {tier} (valid range: 0–5)")]
    InvalidTrustTier { tier: u8 },
}

impl ConstitutionError {
    /// Returns the LawId this error relates to, if applicable.
    pub fn related_law(&self) -> Option<LawId> {
        match self {
            Self::ReceiptMutationAttempt { .. } => Some(LawId::Law1ReceiptImmutability),
            Self::TrustEscalationAttempt { .. } => Some(LawId::Law2IdentityIntegrity),
            Self::IdentityImpersonation { .. } => Some(LawId::Law2IdentityIntegrity),
            Self::MemoryOverwriteWithoutProvenance => Some(LawId::Law3MemorySovereignty),
            Self::ConstitutionRuntimeAccess { .. } => Some(LawId::Law4ConstitutionProtection),
            Self::AnimusBusViolation { .. } => Some(LawId::Law5AnimusIntegrity),
            Self::MultiplePrincipals { .. } => Some(LawId::Law6PrincipalSupremacy),
            Self::OrphanAction { .. } => Some(LawId::Law7CausalChainCompleteness),
            Self::CausalChainIncomplete => Some(LawId::Law7CausalChainCompleteness),
            Self::LawViolation { law, .. } => Some(law.clone()),
            _ => None,
        }
    }

    /// Returns true if this error is a hard block (action must not proceed).
    pub fn is_hard_block(&self) -> bool {
        // All constitutional errors are hard blocks.
        // There is no "soft" constitutional violation.
        true
    }
}
