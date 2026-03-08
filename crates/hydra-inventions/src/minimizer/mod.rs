pub mod compressor;
pub mod dedup;
pub mod reference;

pub use compressor::{CompressionLevel, CompressionResult, ContextCompressor};
pub use dedup::{DedupResult, SemanticDedup};
pub use reference::{ReferenceSubstitution, SubstitutionMap};
