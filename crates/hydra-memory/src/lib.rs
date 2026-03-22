//! `hydra-memory` — The soul of Hydra.
//!
//! Connects the kernel to AgenticMemory via the HydraMemoryBridge.
//!
//! WRITE-AHEAD GUARANTEE:
//!   Every exchange is stored verbatim in AgenticMemory
//!   before Hydra sends its response.
//!   Memory is never lost — not even for the last exchange.
//!
//! SHA256 INTEGRITY:
//!   Every finalized verbatim record has a SHA256 hash.
//!   Hash is verified on every retrieval.
//!   Tampered records are detected immediately.
//!
//! EIGHT MEMORY LAYERS:
//!   Verbatim, Episodic, Semantic, Relational, Causal,
//!   Procedural, Anticipatory, Identity.
//!   All stored as CognitiveEvents in AgenticMemory.
//!
//! 20-YEAR GUARANTEE:
//!   "What did I ask at noon on March 19, 2026?"
//!   Answer: < 50ms via the Chrono-Spatial B+ tree bridge.

pub mod bridge;
pub mod constants;
pub mod errors;
pub mod health;
pub mod identity;
pub mod layers;
pub mod query;
pub mod session;
pub mod temporal_bridge;
pub mod verbatim;

pub use bridge::{HydraMemoryBridge, MemoryHealth};
pub use errors::MemoryError;
pub use health::MemoryHealthSnapshot;
pub use identity::IdentityProfile;
pub use layers::{MemoryLayer, MemoryRecord};
pub use query::{
    query_causal_root, query_exact_timestamp, query_most_recent, query_time_range, QueryResult,
};
pub use session::{SessionManager, SessionRecord};
pub use temporal_bridge::TemporalBridge;
pub use verbatim::{ContextSnapshot, Surface, VerbatimRecord};
