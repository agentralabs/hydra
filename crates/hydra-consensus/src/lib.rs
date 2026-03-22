//! `hydra-consensus` — Cross-instance belief resolution.
//!
//! Instance A believes X. Instance B believes ¬X.
//! Neither simply overwrites the other.
//! Evidence quality and calibration both matter.
//! The merged belief carries provenance from both.
//!
//! AGM belief revision — extended to two agents.

pub mod arbiter;
pub mod belief;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod resolution;

pub use arbiter::{arbitrate, LocalBelief};
pub use belief::SharedBelief;
pub use engine::ConsensusEngine;
pub use errors::ConsensusError;
pub use resolution::{ConsensusResolution, ResolutionMethod};
