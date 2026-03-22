//! `hydra-attention` — Cognitive budget allocation.
//!
//! Hydra focuses on what matters. This crate scores context items,
//! computes an attention budget from intent and affect, and allocates
//! processing depth (Full / Summary / Filtered) within that budget.

pub mod allocator;
pub mod budget;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod frame;
pub mod loop_bridge;
pub mod scorer;

pub use allocator::{allocate, AllocatedItem};
pub use budget::{AttentionBudget, ProcessingDepth};
pub use engine::AttentionEngine;
pub use errors::AttentionError;
pub use frame::AttentionFrame;
pub use scorer::{score_all_items, ScoredItem};
