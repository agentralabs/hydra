//! All constants for hydra-animus.
//! No magic values anywhere else in this crate.

/// Animus Prime binary format magic header — 4 bytes, matches constitution.
pub const ANIMUS_MAGIC: &[u8; 4] = b"ANMA";

/// Animus Prime binary format version.
pub const ANIMUS_VERSION: u32 = 0x00_01_00_00;

/// Maximum number of nodes in a single Prime graph.
pub const PRIME_GRAPH_MAX_NODES: usize = 100_000;

/// Maximum number of edges in a single Prime graph.
pub const PRIME_GRAPH_MAX_EDGES: usize = 500_000;

/// Maximum depth of a causal ⊗-chain before considered malformed.
pub const SIGNAL_CHAIN_MAX_DEPTH: usize = 10_000;

/// Maximum number of signals buffered in the bus before backpressure.
pub const BUS_BUFFER_CAPACITY: usize = 8_192;

/// Signal weight floor — signals below this weight are dropped as noise.
pub const SIGNAL_WEIGHT_FLOOR: f64 = 0.001;

/// Signal weight ceiling — no signal exceeds this.
pub const SIGNAL_WEIGHT_CEILING: f64 = 1.0;

/// Weight coefficient for trust tier contribution to signal weight.
pub const SIGNAL_WEIGHT_ALPHA: f64 = 0.40;

/// Weight coefficient for causal depth contribution to signal weight.
pub const SIGNAL_WEIGHT_BETA: f64 = 0.25;

/// Weight coefficient for novelty contribution to signal weight.
pub const SIGNAL_WEIGHT_GAMMA: f64 = 0.20;

/// Weight coefficient for constitutional relevance.
pub const SIGNAL_WEIGHT_DELTA: f64 = 0.15;

/// Semiring multiplicative identity — the constitutional root.
/// Every ⊗-chain must terminate here.
pub const SEMIRING_IDENTITY_ID: &str = "00000000-0000-0000-0000-000000000001";

/// Semiring additive identity — the null signal ID.
pub const SEMIRING_ZERO_ID: &str = "00000000-0000-0000-0000-000000000000";

/// Maximum domain vocabulary entries per domain.
pub const DOMAIN_VOCAB_MAX_ENTRIES: usize = 10_000;

/// Maximum number of simultaneously registered domains.
pub const DOMAIN_MAX_COUNT: usize = 256;

/// Binary header size in bytes (magic + version + flags).
pub const BINARY_HEADER_SIZE: usize = 12;

/// Ed25519 signature size in bytes.
pub const SIGNATURE_SIZE: usize = 64;

/// Ed25519 public key size in bytes.
pub const PUBLIC_KEY_SIZE: usize = 32;

/// Number of trust tiers (from hydra-constitution, duplicated for weight computation).
pub const TRUST_TIER_COUNT: u8 = 6;

/// Minimum weight boost for growth layer signals.
/// Growth signals are never dropped as noise — they always get at least this weight.
pub const GROWTH_SIGNAL_MIN_BOOST: f64 = 0.5;
