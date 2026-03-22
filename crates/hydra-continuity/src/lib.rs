//! `hydra-continuity` — Morphic signature persistence.
//!
//! The entity's arc made visible. Continuity checkpoints prove
//! that the entity running today is the same entity that started
//! on day one. Yearly checkpoints, lineage proofs, and succession
//! verification — the morphic signature never breaks.

pub mod arc;
pub mod checkpoint;
pub mod constants;
pub mod engine;
pub mod errors;

pub use arc::EntityArc;
pub use checkpoint::ContinuityCheckpoint;
pub use engine::ContinuityEngine;
pub use errors::ContinuityError;
