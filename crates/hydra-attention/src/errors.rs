//! Error types for the hydra-attention crate.

use thiserror::Error;

/// Errors that can occur during attention allocation.
#[derive(Debug, Error)]
pub enum AttentionError {
    /// The attention budget has been fully consumed.
    #[error("attention budget exhausted: {0}")]
    BudgetExhausted(String),

    /// The context provided was empty — nothing to attend to.
    #[error("empty context: no items to allocate attention to")]
    EmptyContext,
}
