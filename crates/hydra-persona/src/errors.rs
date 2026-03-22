//! Error types for hydra-persona.

use thiserror::Error;

/// Errors that can occur during persona operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum PersonaError {
    /// The persona registry is full.
    #[error("Persona registry full: {count}/{max} personas")]
    RegistryFull {
        /// Current count.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },

    /// The requested persona was not found.
    #[error("Persona not found: {name}")]
    PersonaNotFound {
        /// The persona name that was not found.
        name: String,
    },

    /// The blend weights do not sum to 1.0 within tolerance.
    #[error("Invalid blend weights: sum={sum}, expected 1.0 (tolerance {tolerance})")]
    InvalidBlendWeights {
        /// The actual sum of weights.
        sum: f64,
        /// The tolerance used.
        tolerance: f64,
    },

    /// Too many personas in a blend.
    #[error("Blend too large: {count}/{max} personas")]
    BlendTooLarge {
        /// Number of personas requested.
        count: usize,
        /// Maximum allowed in a blend.
        max: usize,
    },
}
