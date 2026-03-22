//! `hydra-redteam` -- Proactive adversarial simulation.
//!
//! NOT reactive (that is hydra-adversary).
//! BEFORE we act: what would an intelligent attacker do?
//!
//! Context + primitives -> threat model -> attack surfaces ->
//! Go / Go-with-mitigations / No-Go.
//! The answer BEFORE the mistake -- not after.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod scenario;
pub mod surface;
pub mod threat;

pub use engine::RedTeamEngine;
pub use errors::RedTeamError;
pub use scenario::{GoNoGo, RedTeamScenario};
pub use surface::{identify_surfaces, AttackSurface};
pub use threat::{threats_from_primitives, ThreatVector};
