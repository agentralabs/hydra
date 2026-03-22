//! `hydra-resurrection` — Continuous delta-state serialization.
//!
//! Hydra never starts cold. This crate provides:
//! - Full and delta checkpoints with SHA256 integrity verification
//! - A checkpoint index that tracks all checkpoints
//! - A writer that decides between full and delta based on index state
//! - A reader that reconstructs state from checkpoints, skipping corrupted ones
//! - A warm restart function that measures reconstruction time

pub mod checkpoint;
pub mod constants;
pub mod errors;
pub mod index;
pub mod reader;
pub mod restart;
pub mod writer;

pub use checkpoint::{Checkpoint, CheckpointKind, KernelStateSnapshot, TaskDelta};
pub use errors::ResurrectionError;
pub use index::CheckpointIndex;
pub use reader::{CheckpointReader, ReconstructedState};
pub use restart::{warm_restart, RestartResult};
pub use writer::CheckpointWriter;
