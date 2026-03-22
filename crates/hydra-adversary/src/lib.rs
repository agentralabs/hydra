//! `hydra-adversary` — The immune system for Hydra.
//!
//! Provides antibodies, threat ecology, and antifragile resistance.
//! Antibodies are NEVER deleted. Antifragile records ONLY grow.
//! Constitutional threats always trigger maximum response.

pub mod antibody;
pub mod antifragile;
pub mod constants;
pub mod ecology;
pub mod errors;
pub mod immune;
pub mod threat;

pub use antibody::Antibody;
pub use antifragile::{AntifragileRecord, AntifragileStore};
pub use ecology::{to_axiom_primitive, ThreatActor, ThreatEcology};
pub use errors::AdversaryError;
pub use immune::{ImmuneAction, ImmuneResponse, ImmuneSystem};
pub use threat::{ThreatClass, ThreatSignal};
