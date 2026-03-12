pub mod advanced;
pub mod boundary;
pub mod challenge;
pub mod gate;
pub mod gate_types;
pub mod kill_switch;
pub mod risk;
pub mod security_layers;

pub use advanced::{harm_predict, shadow_sim, SimOutcome, SimResult};
pub use boundary::{BoundaryEnforcer, BoundaryResult, BoundaryViolation, HardBoundary};
pub use gate::{ExecutionGate, GateConfig, GateDecision};
pub use kill_switch::KillSwitch;
pub use risk::{ActionContext, BlastRadius, RiskAssessor};
pub use challenge::{ChallengeManager, ChallengePhrase};
pub use security_layers::{GateAuditEntry, PerimeterConfig, ResourceLimits, SessionContext};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_gate;
#[cfg(test)]
mod tests_security;
