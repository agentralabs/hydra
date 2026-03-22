//! `hydra-horizon` — Perception and action horizon.
//!
//! Tracks Hydra's expanding awareness (perception) and ability to act
//! (action). Horizons only expand, never contract. The combined horizon
//! is the geometric mean of perception and action.

pub mod action;
pub mod constants;
pub mod errors;
pub mod horizon;
pub mod perception;

pub use action::{ActionExpansion, ActionHorizon};
pub use errors::HorizonError;
pub use horizon::Horizon;
pub use perception::{PerceptionExpansion, PerceptionHorizon};
