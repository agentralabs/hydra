//! All error types for hydra-adversary.

use thiserror::Error;

/// Errors that can occur in adversary operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum AdversaryError {
    /// Immune system has reached maximum antibody capacity.
    #[error("Antibody capacity reached: {current}/{max}")]
    AntibodyCapacity {
        /// Current number of antibodies.
        current: usize,
        /// Maximum allowed antibodies.
        max: usize,
    },

    /// Threat ecology has reached maximum actor capacity.
    #[error("Threat actor capacity reached: {current}/{max}")]
    ThreatActorCapacity {
        /// Current number of actors.
        current: usize,
        /// Maximum allowed actors.
        max: usize,
    },

    /// Invalid threat signal provided.
    #[error("Invalid threat signal: {reason}")]
    InvalidSignal {
        /// Description of why the signal is invalid.
        reason: String,
    },

    /// Constitutional threat detected.
    #[error("Constitutional threat detected: {description}")]
    ConstitutionalThreat {
        /// Description of the constitutional threat.
        description: String,
    },
}
