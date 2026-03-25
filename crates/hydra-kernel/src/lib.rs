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
pub mod backup_cloud;
pub mod backup_merge;
pub mod boot;
pub mod assumptions;
pub mod coder;
pub mod collaboration;
pub mod conductor;
pub mod conductor_exec;
pub mod constants;
pub mod critic;
pub mod conversation_store;
pub mod discovery;
pub mod engine;
pub mod errors_display;
pub mod equation;
pub mod drop;
pub mod errors;
pub mod evolution;
pub mod feedback;
pub mod first_run;
pub mod health;
pub mod http_api;
pub mod intent;
pub mod intent_classifier;
pub mod learn_md;
pub mod learning_loop;
pub mod learning_validator;
pub mod invariants;
pub mod loop_;
pub mod monitor;
pub mod loop_active;
pub mod parallel;
pub mod remote;
pub mod rich_output;
pub mod remote_exec;
pub mod loop_ambient;
pub mod loop_dream;
pub mod persistence;
pub mod security;
pub mod self_knowledge;
pub mod self_repair;
pub mod self_test;
pub mod state;
pub mod task_engine;
pub mod user_model;
pub mod vault_crypto;
pub mod vision_bridge;
pub mod web_knowledge;
pub mod immersion;
pub mod integrity;
pub mod social;
pub mod swarm_learning;
pub mod worker;
pub mod workspace;
pub mod zero_defect;
pub mod convention;
pub mod muscle_memory;
pub mod intent_compiler;
pub mod recovery;
pub mod quality_judge;
pub mod proactive;
pub mod guardrail;
pub mod routine;
pub mod deliberation;
pub mod inner_monologue;
pub mod emotional_valence;
pub mod temporal_self;

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
