//! Settlement error types.

use thiserror::Error;

/// Errors that can occur during settlement operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SettlementError {
    #[error("Settlement ledger at capacity ({max})")]
    LedgerFull { max: usize },

    #[error("Record '{id}' already settled — immutable")]
    AlreadySettled { id: String },

    #[error("No records found for period {start} to {end}")]
    EmptyPeriod { start: String, end: String },

    #[error("Task '{task_id}' not found in settlement ledger")]
    TaskNotFound { task_id: String },
}
