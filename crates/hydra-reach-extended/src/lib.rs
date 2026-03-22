//! `hydra-reach-extended` — External system connectivity.
//!
//! Any server. Any API. Any network target.
//! Multi-path connectivity with relentless approach escalation.
//! FAILED does not exist for connectivity.
//! Hard denial only on explicit credential rejection.
//! Every path attempt receipted.
//! Cartography grows with every new system encountered.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod path;
pub mod resolver;
pub mod session;
pub mod target;

pub use engine::{ReachEngine, ReachResult};
pub use errors::ReachError;
pub use path::{ConnectionPath, PathOutcome, PathType};
pub use resolver::PathResolver;
pub use session::{ReachSession, SessionState};
pub use target::{ReachTarget, TargetClass};
