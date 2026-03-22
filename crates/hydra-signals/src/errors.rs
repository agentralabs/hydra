//! Error types for hydra-signals.

use thiserror::Error;

/// Errors that can occur in the signal fabric.
#[derive(Debug, Error, Clone)]
pub enum SignalError {
    /// Signal rejected at gate — orphan chain.
    #[error("Signal '{id}' rejected at gate: orphan (causal chain incomplete)")]
    OrphanRejected {
        /// The signal ID that was rejected.
        id: String,
    },

    /// Signal rejected — chain too deep.
    #[error("Signal '{id}' rejected: causal chain depth {depth} exceeds maximum {max}")]
    ChainTooDeep {
        /// The signal ID that was rejected.
        id: String,
        /// The actual chain depth.
        depth: usize,
        /// The maximum allowed depth.
        max: usize,
    },

    /// No handler registered for tier.
    #[error("No handler registered for tier {tier:?}")]
    NoHandlerForTier {
        /// The tier name.
        tier: String,
    },

    /// Queue full — backpressure.
    #[error("Queue full for tier {tier:?}: capacity {capacity}")]
    QueueFull {
        /// The tier name.
        tier: String,
        /// The queue capacity.
        capacity: usize,
    },

    /// Subscriber registration failed.
    #[error("Subscriber registration failed for topic '{topic}': {reason}")]
    SubscriptionFailed {
        /// The topic that failed.
        topic: String,
        /// The reason for failure.
        reason: String,
    },

    /// Signal delivery failed.
    #[error("Delivery failed for signal '{id}' to handler '{handler}': {reason}")]
    DeliveryFailed {
        /// The signal ID.
        id: String,
        /// The handler name.
        handler: String,
        /// The reason for failure.
        reason: String,
    },

    /// Fabric not initialized.
    #[error("Signal fabric not initialized — call SignalFabric::new() first")]
    FabricNotInitialized,

    /// Constitutional violation in signal processing.
    #[error("Constitutional violation while processing signal '{id}': {reason}")]
    ConstitutionalViolation {
        /// The signal ID.
        id: String,
        /// The reason for the violation.
        reason: String,
    },
}
