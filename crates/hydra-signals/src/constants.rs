//! All constants for hydra-signals.
//! No magic values anywhere else in this crate.

/// Queue capacity for each signal tier.
pub const QUEUE_CAPACITY_CONSTITUTION: usize = 64;
/// Queue capacity for adversarial signals.
pub const QUEUE_CAPACITY_ADVERSARIAL: usize = 512;
/// Queue capacity for belief revision signals.
pub const QUEUE_CAPACITY_BELIEF_REVISION: usize = 2_048;
/// Queue capacity for fleet signals.
pub const QUEUE_CAPACITY_FLEET: usize = 8_192;
/// Queue capacity for companion signals.
pub const QUEUE_CAPACITY_COMPANION: usize = 4_096;
/// Queue capacity for prediction signals.
pub const QUEUE_CAPACITY_PREDICTION: usize = 4_096;

/// Maximum subscribers per topic.
pub const MAX_SUBSCRIBERS_PER_TOPIC: usize = 64;

/// Maximum topics in the subscription registry.
pub const MAX_TOPICS: usize = 1_024;

/// Signal audit trail maximum entries before rotation.
pub const AUDIT_TRAIL_MAX_ENTRIES: usize = 100_000;

/// Backpressure threshold — fraction of queue capacity.
pub const BACKPRESSURE_THRESHOLD: f64 = 0.85;

/// Maximum time a signal may wait in a queue before escalation (ms).
pub const SIGNAL_ESCALATION_TIMEOUT_MS: u64 = 5_000;

/// Maximum causal chain depth accepted at the entry gate.
pub const GATE_MAX_CHAIN_DEPTH: usize = 10_000;

/// Delivery receipt retention window (seconds).
pub const RECEIPT_HOT_RETENTION_SECONDS: u64 = 3_600;
