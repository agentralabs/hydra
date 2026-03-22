//! `hydra-trust` — Trust thermodynamics for the Hydra fleet.
//!
//! Models trust as a thermodynamic quantity using a Hamiltonian formulation.
//! Agents accumulate trust through successes and lose it through failures.
//! Constitutional violations trigger immediate phase transitions.

pub mod agent;
pub mod constants;
pub mod errors;
pub mod field;
pub mod hamiltonian;
pub mod score;
pub mod spawn;

pub use agent::{AgentState, TrustAgent};
pub use errors::TrustError;
pub use field::TrustField;
pub use hamiltonian::{apply_violation_spike, compute_hamiltonian, HamiltonianState, TrustPhase};
pub use score::{TrustScore, TrustTier};
pub use spawn::{boltzmann_weight, spawn_decision};
