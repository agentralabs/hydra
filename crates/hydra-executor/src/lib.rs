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
pub mod bridge;
pub mod bridge_config;
pub mod bridge_process;
pub mod file_ops;
pub mod integration;
pub mod local_config;
pub mod local_executor;
pub mod runtime;

pub use action_loader::{Action, ActionRegistry as ExternalActionRegistry};
pub use engine::{ExecutionEngine, ExecutionRequest};
pub use integration::{Integration, IntegrationRegistry};
pub use runtime::{ExecutionResult, execute_shell, execute_api_sync};
pub use errors::ExecutorError;
pub use receipt::{ExecutionReceipt, ReceiptLedger, ReceiptOutcome};
pub use registry::{ActionRegistry, ExecutorType, RegisteredAction};
pub use task::{ApproachType, TaskRecord, TaskState};
pub use bridge::BridgeManager;
pub use bridge_config::BridgeConfig;
pub use bridge_process::{BridgeProcess, BridgeSignal, BridgeState};
pub use local_config::LocalConfig;
pub use local_executor::LocalExecutor;
