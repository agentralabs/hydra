//! `hydra-antifragile` — Obstacle resistance.
//!
//! Hydra grows stronger from every obstacle encountered.
//! Resistance records track how well Hydra navigates each class
//! of obstacle, and resistance only grows — never decreases.

pub mod constants;
pub mod errors;
pub mod obstacle;
pub mod record;
pub mod store;

pub use errors::AntifragileError;
pub use obstacle::{ObstacleClass, ObstacleSignature};
pub use record::ResistanceRecord;
pub use store::AntifragileStore;
