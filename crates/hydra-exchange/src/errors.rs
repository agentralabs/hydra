//! Error types for the exchange subsystem.

use thiserror::Error;

/// Errors that can occur during exchange operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ExchangeError {
    #[error("Trust score {score:.2} below minimum {min:.2} for counterparty '{counterparty}'")]
    InsufficientTrust {
        counterparty: String,
        score: f64,
        min: f64,
    },

    #[error("Wisdom confidence {confidence:.2} below minimum {min:.2} — exchange deferred")]
    InsufficientWisdom { confidence: f64, min: f64 },

    #[error("Offer '{offer_id}' not found in registry")]
    OfferNotFound { offer_id: String },

    #[error(
        "Exchange value {value:.1} exceeds unescalated threshold {max:.1} — escalation required"
    )]
    EscalationRequired { value: f64, max: f64 },

    #[error("Capability '{capability}' not available in current portfolio")]
    CapabilityUnavailable { capability: String },

    #[error("Exchange registry at capacity ({max})")]
    RegistryFull { max: usize },
}

impl ExchangeError {
    /// Whether this error requires human intervention to resolve.
    pub fn requires_human(&self) -> bool {
        matches!(
            self,
            Self::EscalationRequired { .. } | Self::InsufficientTrust { .. }
        )
    }
}
