//! `hydra-executor` — Universal action engine.
//!
//! FAILED does not exist as a state. Ever.
//! Every obstacle is navigational.
//! Every execution is receipted before it starts.
//! 13 approach types before HardDenied.
//! HardDenied requires explicit evidence.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod receipt;
pub mod registry;
pub mod runner;
pub mod task;

pub mod action_loader;
pub mod integration;
pub mod runtime;

pub use action_loader::{Action, ActionRegistry as ExternalActionRegistry};
pub use engine::{ExecutionEngine, ExecutionRequest};
pub use integration::{Integration, IntegrationRegistry};
pub use runtime::{ExecutionResult, execute_shell, execute_api_sync};
pub use errors::ExecutorError;
pub use receipt::{ExecutionReceipt, ReceiptLedger, ReceiptOutcome};
pub use registry::{ActionRegistry, ExecutorType, RegisteredAction};
pub use task::{ApproachType, TaskRecord, TaskState};
