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

pub mod boot;
pub mod constants;
pub mod engine;
pub mod equation;
pub mod errors;
pub mod health;
pub mod intent;
pub mod invariants;
pub mod loop_;
pub mod loop_active;
pub mod loop_ambient;
pub mod loop_dream;
pub mod persistence;
pub mod self_knowledge;
pub mod self_repair;
pub mod state;
pub mod task_engine;
pub mod web_knowledge;

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
