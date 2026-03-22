//! Kernel error types.

use thiserror::Error;

/// All errors that can occur within the hydra-kernel.
#[derive(Debug, Error, Clone)]
pub enum KernelError {
    /// Boot sequence failed.
    #[error("Boot failed at phase '{phase}': {reason}")]
    BootFailed {
        /// Which boot phase failed.
        phase: String,
        /// Why it failed.
        reason: String,
    },

    /// Constitutional violation during kernel operation.
    #[error("Constitutional violation detected during kernel operation: {reason}")]
    ConstitutionalViolation {
        /// Description of the violation.
        reason: String,
    },

    /// Invariant check failed.
    #[error("Invariant check failed: {invariant}")]
    InvariantFailed {
        /// Which invariant failed.
        invariant: String,
    },

    /// Animus bus unreachable.
    #[error("Animus bus unreachable: {reason}")]
    AnimusBusUnreachable {
        /// Why the bus is unreachable.
        reason: String,
    },

    /// Task engine error.
    #[error("Task engine error for task '{task_id}': {reason}")]
    TaskEngineError {
        /// Which task encountered the error.
        task_id: String,
        /// What went wrong.
        reason: String,
    },

    /// Signal queue full.
    #[error("Signal queue full on channel '{channel}'")]
    SignalQueueFull {
        /// Which channel is full.
        channel: String,
    },

    /// Kernel not booted.
    #[error("Kernel not yet booted — operation '{op}' requires a running kernel")]
    NotBooted {
        /// Which operation was attempted.
        op: String,
    },

    /// Lyapunov stability critical.
    #[error("Lyapunov stability critical: V(Psi) = {value:.4}")]
    LyapunovCritical {
        /// The current Lyapunov value.
        value: f64,
    },

    /// Shutdown timeout.
    #[error("Shutdown timeout: kernel did not shut down cleanly within {ms}ms")]
    ShutdownTimeout {
        /// How many milliseconds were allowed.
        ms: u64,
    },

    /// Thread panicked.
    #[error("Thread '{name}' panicked: {reason}")]
    ThreadPanic {
        /// Which thread panicked.
        name: String,
        /// Panic message.
        reason: String,
    },
}
