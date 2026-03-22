//! `hydra-portfolio` — Resource allocation.
//!
//! Given what Hydra has spent (settlement) and why (attribution),
//! where should it focus next?
//!
//! Objective scoring: risk * 0.30 + orientation * 0.25 + roi * 0.25 + urgency * 0.20
//! Score-proportional allocation of attention budget.
//! Avoidability report informs cost-reduction objectives.

pub mod allocation;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod objective;
pub mod ranker;

pub use allocation::{AllocationEntry, ResourceAllocation};
pub use engine::PortfolioEngine;
pub use errors::PortfolioError;
pub use objective::{ObjectiveCategory, PortfolioObjective};
pub use ranker::{rank_objectives, ScoredObjective};
