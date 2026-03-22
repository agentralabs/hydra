//! Error types for the noticing engine.

use thiserror::Error;

/// Errors that can occur during noticing operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum NoticingError {
    #[error("Insufficient baseline data ({samples} samples, need {min})")]
    InsufficientBaseline { samples: usize, min: usize },

    #[error("Signal queue at capacity ({max})")]
    QueueFull { max: usize },

    #[error("Metric '{name}' not tracked — register it first")]
    MetricNotTracked { name: String },
}
