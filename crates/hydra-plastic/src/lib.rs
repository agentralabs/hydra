//! `hydra-plastic` — Execution environment adaptation.
//!
//! Tracks execution environments and their confidence levels,
//! enabling Hydra to adapt its execution strategy based on
//! accumulated experience. The tensor is append-only.

pub mod constants;
pub mod errors;
pub mod mode;
pub mod environment;
pub mod tensor;

pub use errors::PlasticError;
pub use mode::ExecutionMode;
pub use environment::EnvironmentProfile;
pub use tensor::PlasticityTensor;
