//! Error types for hydra-axiom.

use thiserror::Error;

/// Errors that can occur in axiom operations.
#[derive(Debug, Error, Clone)]
pub enum AxiomError {
    /// A referenced primitive was not found.
    #[error("Axiom primitive not found: '{name}'")]
    PrimitiveNotFound {
        /// The name of the missing primitive.
        name: String,
    },

    /// A domain with the same name is already registered.
    #[error("Domain already registered: '{domain}'")]
    DomainAlreadyRegistered {
        /// The domain that was already registered.
        domain: String,
    },

    /// Functor composition violates category laws.
    #[error("Functor composition violation: {reason}")]
    FunctorCompositionViolation {
        /// Why the composition failed.
        reason: String,
    },

    /// The functor registry is full.
    #[error("Functor registry full (max {max} domains)")]
    RegistryFull {
        /// The maximum number of domains.
        max: usize,
    },

    /// Synthesis requires at least one primitive component.
    #[error("Synthesis missing primitive: cannot synthesize from empty components")]
    SynthesisMissingPrimitive,
}
