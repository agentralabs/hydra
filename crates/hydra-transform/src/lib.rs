//! `hydra-transform` — Any data, any format, meaning preserved.
//!
//! Skills register format vocabularies here.
//! All conversions go through a universal intermediate.
//! Meaning is preserved — not just structure.
//! Sister output -> Animus Prime -> reasoning substrate.

pub mod constants;
pub mod converter;
pub mod engine;
pub mod errors;
pub mod format;
pub mod registry;

pub use converter::{convert, ConversionResult};
pub use engine::TransformEngine;
pub use errors::TransformError;
pub use format::DataFormat;
pub use registry::{FormatRegistry, FormatVocabulary};
