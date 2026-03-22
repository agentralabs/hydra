//! Error types for hydra-genome.

use thiserror::Error;

/// Errors that can occur in genome operations.
#[derive(Debug, Error, Clone)]
pub enum GenomeError {
    /// The genome store has reached its maximum capacity.
    #[error("Genome store full (max {max} entries)")]
    StoreFull {
        /// The maximum number of entries allowed.
        max: usize,
    },

    /// A referenced genome entry was not found.
    #[error("Genome entry not found: '{id}'")]
    EntryNotFound {
        /// The ID of the missing entry.
        id: String,
    },

    /// Confidence value is out of bounds.
    #[error("Confidence out of bounds: {value} (must be 0.0..=1.0)")]
    ConfidenceOutOfBounds {
        /// The invalid confidence value.
        value: f64,
    },
}
