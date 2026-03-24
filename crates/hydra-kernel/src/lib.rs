//! `hydra-kernel` — The alive loop. Hydra's phenomenological core.
//!
//! This crate implements the three concurrent loops that make Hydra alive:
//!
//! - **ACTIVE**: Foreground processing of principal commands
//! - **AMBIENT**: Background maintenance, invariant checking, equation integration
//! - **DREAM**: Idle-period belief consolidation and prediction rehearsal
//!
//! The kernel equation: `dPsi/dt = L-hat Psi + A-hat Psi + G-hat Psi + S-hat Psi - Gamma-hat Psi`
//!
//! All state is immutable — each tick produces a new `HydraState`.
//! Constitutional invariants are checked on every ambient tick.

pub mod backup;
pub mod boot;
pub mod assumptions;
pub mod coder;
pub mod conductor;
pub mod conductor_exec;
pub mod constants;
pub mod critic;
pub mod conversation_store;
pub mod engine;
pub mod errors_display;
pub mod equation;
pub mod errors;
pub mod feedback;
pub mod first_run;
pub mod health;
pub mod http_api;
pub mod intent;
pub mod intent_classifier;
pub mod learning_loop;
pub mod learning_validator;
pub mod invariants;
pub mod loop_;
pub mod loop_active;
pub mod parallel;
pub mod loop_ambient;
pub mod loop_dream;
pub mod persistence;
pub mod self_knowledge;
pub mod self_repair;
pub mod self_test;
pub mod state;
pub mod task_engine;
pub mod vault_crypto;
pub mod vision_bridge;
pub mod web_knowledge;
pub mod social;
pub mod worker;
pub mod workspace;
pub mod zero_defect;

// Re-exports for convenience
pub use boot::{BootResult, run_boot_sequence};
pub use constants::KERNEL_VERSION;
pub use errors::KernelError;
pub use health::KernelHealth;
pub use loop_active::{ActiveCommand, ActiveResult, process_command};
pub use loop_ambient::{AmbientSubsystems, AmbientTickResult, tick as ambient_tick};
pub use loop_dream::{DreamCycleResult, DreamSubsystems, cycle as dream_cycle};
pub use state::{HydraState, KernelPhase};
pub use task_engine::{ManagedTask, TaskEngine};
