//! `hydra-environment` — Any environment. Constraints declared. Execution adapts.
//!
//! Skills declare what they need.
//! The engine detects what exists.
//! The checker bridges them.
//! The executor never needs to know where it is running.

pub mod checker;
pub mod constants;
pub mod detector;
pub mod engine;
pub mod errors;
pub mod profile;
pub mod requirements;

pub use checker::{check_requirements, CheckOutcome};
pub use detector::EnvironmentDetector;
pub use engine::EnvironmentEngine;
pub use errors::EnvironmentError;
pub use profile::{EnvironmentCapabilities, EnvironmentClass, EnvironmentProfile, OsType};
pub use requirements::{RequiredBinary, SkillRequirements};
