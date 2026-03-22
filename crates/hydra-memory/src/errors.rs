//! Error types for hydra-memory.

use thiserror::Error;

/// Errors that can occur in hydra-memory operations.
#[derive(Debug, Error, Clone)]
pub enum MemoryError {
    /// AgenticMemory is unavailable — kernel must halt.
    #[error("AgenticMemory unavailable: {reason}. Memory is non-optional — kernel halting.")]
    AgenticMemoryUnavailable {
        /// The reason AgenticMemory is unavailable.
        reason: String,
    },

    /// Write-ahead failed — response must not be sent.
    #[error("Write-ahead failed for exchange '{exchange_id}': {reason}")]
    WriteAheadFailed {
        /// The exchange ID that failed.
        exchange_id: String,
        /// The reason the write-ahead failed.
        reason: String,
    },

    /// SHA256 integrity check failed on retrieval.
    #[error("Integrity check failed for verbatim record '{id}': hash mismatch")]
    IntegrityCheckFailed {
        /// The ID of the record that failed integrity check.
        id: String,
    },

    /// Verbatim record not found.
    #[error("Verbatim record not found: '{id}'")]
    VerbatimNotFound {
        /// The ID of the record that was not found.
        id: String,
    },

    /// Memory layer mapping failed.
    #[error("Failed to map to memory layer '{layer}': {reason}")]
    LayerMappingFailed {
        /// The layer that failed.
        layer: String,
        /// The reason it failed.
        reason: String,
    },

    /// AgenticMemory write error.
    #[error("AgenticMemory write error: {reason}")]
    WriteError {
        /// The reason for the write error.
        reason: String,
    },

    /// AgenticMemory query error.
    #[error("AgenticMemory query error: {reason}")]
    QueryError {
        /// The reason for the query error.
        reason: String,
    },

    /// Identity memory insufficient data.
    #[error("Identity memory: insufficient data ({sessions} sessions, need {required})")]
    InsufficientIdentityData {
        /// The number of sessions observed.
        sessions: usize,
        /// The number required.
        required: usize,
    },

    /// Session boundary error.
    #[error("Session boundary error: {reason}")]
    SessionError {
        /// The reason for the session error.
        reason: String,
    },
}
