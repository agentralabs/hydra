//! The Signal Causal Semiring — (S, +, *, 0, 1)

pub mod compose;
pub mod merge;
pub mod orphan;
pub mod signal;
pub mod weight;

pub use compose::compose;
pub use merge::merge;
pub use orphan::{is_orphan, validate_chain};
pub use signal::{Signal, SignalId, SignalTier, SignalWeight};
pub use weight::{compute_weight, verify_coefficient_sum, WeightInputs};
