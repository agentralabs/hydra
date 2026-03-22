//! `hydra-fleet` — Agent lifecycle management.
//!
//! Constitutional spawning, task assignment, result receipting, and
//! quarantine enforcement for the Hydra fleet.

pub mod agent;
pub mod assignment;
pub mod constants;
pub mod errors;
pub mod registry;
pub mod result;
pub mod spawn;
pub mod task;

pub use agent::{AgentSpecialization, FleetAgent, FleetAgentState};
pub use assignment::{find_agent, preferred_specialization, AssignmentStrategy};
pub use errors::FleetError;
pub use registry::FleetRegistry;
pub use result::{AgentResult, ResultOutcome, ResultReceipt};
pub use spawn::{check_spawn, SpawnCheckResult, SpawnRequest};
pub use task::{FleetTask, TaskType};
