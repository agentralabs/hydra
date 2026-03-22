//! `hydra-soul` — The orientation layer. Hydra knows what the work is for.
//!
//! The soul accumulates meaning from every exchange and, once it has
//! enough data, provides orientation context alongside output without
//! ever changing the content itself.
//!
//! Key invariants:
//! - The MeaningGraph is append-only (no deletes, no resets).
//! - OrientedOutput never changes content — only adds context alongside.
//! - All external writes go through `Soul::record_exchange()`.
//! - Constitutional deepening enforces a minimum reflection period.

pub mod constants;
pub mod deepening;
pub mod errors;
pub mod graph;
pub mod node;
pub mod orient;
pub mod soul;
pub mod temporal;

pub use deepening::{DeepeningRecord, DeepeningState, DeepeningStore};
pub use errors::SoulError;
pub use graph::MeaningGraph;
pub use node::{MeaningNode, NodeKind};
pub use orient::{orientation_summary, OrientationContext, OrientedOutput};
pub use soul::Soul;
pub use temporal::{TemporalHorizon, TemporalSignals};
