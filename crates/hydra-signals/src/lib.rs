//! `hydra-signals` — The live signal fabric for Hydra.
//!
//! Zero-loss routing between Hydra modules. Every inter-module signal
//! flows through this fabric, which enforces constitutional compliance,
//! priority-based routing, delivery receipts, and full audit trails.
//!
//! ## Architecture
//!
//! ```text
//! Signal → Gate → Router → Queue (by tier) → Dispatch → Subscribers
//!                    ↓                            ↓
//!              Audit Trail                  Delivery Receipts
//! ```
//!
//! Constitutional signals bypass queues entirely and are handled immediately.
//! Orphan signals (incomplete causal chains) are rejected at the gate.

pub mod audit;
pub mod companion_channel;
pub mod constants;
pub mod dispatch;
pub mod errors;
pub mod fabric;
pub mod gate;
pub mod queue;
pub mod receipt;
pub mod router;
pub mod subscription;
pub mod weight;

// Top-level re-exports for convenience
pub use audit::{AuditAction, AuditEntry, SignalAuditTrail};
pub use errors::SignalError;
pub use fabric::{FabricStatus, SignalFabric};
pub use gate::SignalGate;
pub use queue::{SignalQueues, TierQueue};
pub use receipt::{DeliveryOutcome, DeliveryReceipt, DeliveryReceiptId, DeliveryReceiptLog};
pub use router::{fabric_route, signal_topic, FabricRoute};
pub use subscription::{SubscriberId, Subscription, SubscriptionRegistry};
pub use weight::compute_signal_weight;
pub use companion_channel::{
    CompanionChannel, CompanionCommand, CompanionEndpoint, CompanionOutput,
    SignalClass as CompanionSignalClass, create_channel as create_companion_channel,
};
