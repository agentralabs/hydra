//! All constants for hydra-memory.
//! No magic values anywhere else in this crate.

/// AgenticMemory file path (relative to Hydra root).
/// This is where the .amem file lives.
pub const AGENTIC_MEMORY_PATH: &str = "data/hydra.amem";

/// SHA256 hash size in bytes.
pub const SHA256_HASH_SIZE: usize = 32;

/// Maximum verbatim input size in bytes (10MB).
pub const MAX_VERBATIM_SIZE_BYTES: usize = 10 * 1024 * 1024;

/// Verbatim records older than this are compressed (days).
pub const VERBATIM_COMPRESSION_AGE_DAYS: u64 = 90;

/// Memory mass floor — fossils never go below this.
pub const MEMORY_MASS_FLOOR: f64 = 0.001;

/// Default memory gravitational constant.
pub const MEMORY_GRAVITY_CONSTANT: f64 = 0.01;

/// Session boundary gap — silence longer than this = new session (minutes).
pub const SESSION_BOUNDARY_GAP_MINUTES: u64 = 30;

/// Maximum exchanges per session before forced boundary.
pub const SESSION_MAX_EXCHANGES: usize = 1_000;

/// Identity memory minimum sessions before profile confidence is high.
pub const IDENTITY_MIN_SESSIONS_FOR_CONFIDENCE: usize = 10;

/// Write-ahead timeout — max ms to wait for memory write before error.
pub const WRITE_AHEAD_TIMEOUT_MS: u64 = 5_000;

/// CognitiveEvent content type tags for the 8 memory layers.
pub const LAYER_VERBATIM: &str = "hydra:verbatim";
/// Episodic layer tag.
pub const LAYER_EPISODIC: &str = "hydra:episodic";
/// Semantic layer tag.
pub const LAYER_SEMANTIC: &str = "hydra:semantic";
/// Relational layer tag.
pub const LAYER_RELATIONAL: &str = "hydra:relational";
/// Causal layer tag.
pub const LAYER_CAUSAL: &str = "hydra:causal";
/// Procedural layer tag.
pub const LAYER_PROCEDURAL: &str = "hydra:procedural";
/// Anticipatory layer tag.
pub const LAYER_ANTICIPATORY: &str = "hydra:anticipatory";
/// Identity layer tag.
pub const LAYER_IDENTITY: &str = "hydra:identity";

/// Session index prefix in AgenticMemory.
pub const SESSION_INDEX_PREFIX: &str = "hydra:session:";

/// Default embedding dimension (matches AgenticMemory DEFAULT_DIMENSION = 128).
/// Must match CognitiveEventBuilder's default or writes fail with
/// "Feature vector dimension mismatch: expected 0, got 128".
pub const EMBEDDING_DIMENSION: usize = 128;
