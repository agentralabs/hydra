//! `hydra-automation` — Behavior crystallization.
//!
//! "You have done this 4 times this week.
//!  I can automate this. Shall I?"
//!
//! Observes executions. Detects patterns.
//! Proposes crystallization. Never auto-crystallizes.
//! On approval: generates valid SKILL-FORMAT-SPEC.md package.
//! Hot-loads the skill. The behavior is permanent.
//! Knowledge survives even if the skill is later unloaded.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod generator;
pub mod observation;
pub mod pattern;
pub mod proposal;

pub use engine::AutomationEngine;
pub use errors::AutomationError;
pub use generator::{GeneratedSkillPackage, SkillGenerator};
pub use observation::ExecutionObservation;
pub use pattern::BehaviorPattern;
pub use proposal::{CrystallizationProposal, ProposalState};
