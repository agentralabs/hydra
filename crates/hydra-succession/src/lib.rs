//! `hydra-succession` — Knowledge transfer across generations.
//!
//! The entity survives the substrate change.
//!
//! Four things that must transfer:
//!   Soul orientation (20-year meaning graph)
//!   Genome (proven approach library)
//!   Calibration profiles (domain bias corrections)
//!   Morphic signature (identity continuity)
//!
//! Three gates before any import:
//!   Integrity (SHA256 hash verified)
//!   Identity (lineage and morphic signature match)
//!   Constitution (soul and genome non-erasable laws)
//!
//! One-time per instance. Immutable once imported.
//! Layer 7 begins here.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod exporter;
pub mod package;
pub mod payload;
pub mod verifier;

pub use engine::{SuccessionEngine, SuccessionResult};
pub use errors::SuccessionError;
pub use exporter::{InstanceState, SuccessionExporter};
pub use package::{PackageState, SuccessionPackage};
pub use payload::{CalibrationPayload, GenomePayload, MorphicPayload, SoulPayload};
pub use verifier::{SuccessionVerifier, VerificationResult};
