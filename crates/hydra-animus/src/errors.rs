//! All error types for hydra-animus.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum AnimusError {
    /// The Prime graph exceeds size limits.
    #[error("Prime graph too large: {nodes} nodes, {edges} edges (max {max_nodes}/{max_edges})")]
    GraphTooLarge {
        nodes: usize,
        edges: usize,
        max_nodes: usize,
        max_edges: usize,
    },

    /// A node referenced in an edge does not exist in the graph.
    #[error("Edge references unknown node: edge '{edge_id}' references '{node_id}'")]
    UnknownNodeReference { edge_id: String, node_id: String },

    /// Orphan signal detected — causal chain is empty or doesn't reach identity.
    #[error("Orphan signal '{signal_id}': causal chain does not reach constitutional identity")]
    OrphanSignal { signal_id: String },

    /// Signal weight is out of valid range.
    #[error("Signal weight {weight} is out of range [{min}, {max}]")]
    InvalidSignalWeight { weight: f64, min: f64, max: f64 },

    /// Binary serialization error.
    #[error("Serialization failed: {reason}")]
    SerializationFailed { reason: String },

    /// Binary deserialization error.
    #[error("Deserialization failed at offset {offset}: {reason}")]
    DeserializationFailed { offset: usize, reason: String },

    /// Invalid Animus magic header.
    #[error("Invalid magic header: expected {expected:?}, got {got:?}")]
    InvalidMagicHeader { expected: Vec<u8>, got: Vec<u8> },

    /// Version mismatch.
    #[error("Version mismatch: expected {expected:#010x}, got {got:#010x}")]
    VersionMismatch { expected: u32, got: u32 },

    /// Ed25519 signature verification failed.
    #[error("Signature verification failed for message '{message_id}'")]
    SignatureVerificationFailed { message_id: String },

    /// Bus channel is full — backpressure triggered.
    #[error("Bus channel full: capacity {capacity} reached for channel '{channel}'")]
    BusChannelFull { capacity: usize, channel: String },

    /// Domain vocabulary registration failed.
    #[error("Domain vocabulary registration failed for domain '{domain}': {reason}")]
    VocabRegistrationFailed { domain: String, reason: String },

    /// Unknown node type — not in base or any registered domain vocabulary.
    #[error("Unknown node type '{type_name}' — not in any registered vocabulary")]
    UnknownNodeType { type_name: String },

    /// Constitutional check failed during signal bus validation.
    #[error("Constitutional violation on signal '{signal_id}': {reason}")]
    ConstitutionalViolation { signal_id: String, reason: String },

    /// Causal chain is malformed.
    #[error("Malformed causal chain for signal '{signal_id}': {reason}")]
    MalformedCausalChain { signal_id: String, reason: String },
}
