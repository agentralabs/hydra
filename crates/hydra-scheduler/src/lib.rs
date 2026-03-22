//! `hydra-scheduler` — Temporal execution.
//!
//! Hydra acts when constraints fire — not just when asked.
//! Recurring jobs. One-shot futures. Constraint activations.
//! Metric condition triggers.
//! Everything receipted before firing.
//! Soul temporal horizon calibrates care level.

pub mod clock;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod job;
pub mod queue;
pub mod trigger;

pub use clock::SchedulerClock;
pub use engine::{SchedulerEngine, TickResult};
pub use errors::SchedulerError;
pub use job::{JobState, ScheduledJob};
pub use queue::JobQueue;
pub use trigger::{MetricConditionType, TriggerType};
