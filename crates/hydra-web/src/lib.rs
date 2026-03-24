//! hydra-web — World-class web access engine for Hydra.
//!
//! Multi-engine search, smart content extraction, learning cache,
//! and optional LLM synthesis. Zero API keys required for core search.
//! Every search makes Hydra smarter — repeat queries are instant.

pub mod cache;
pub mod constants;
pub mod engines;
pub mod errors;
pub mod extractor;
pub mod orchestrator;
pub mod ranker;
pub mod synthesis;
pub mod types;

pub use errors::WebError;
pub use orchestrator::SearchOrchestrator;
pub use types::{
    CachePolicy, ContentFocus, EngineLabel, ExtractedContent, SearchHit, WebSearchRequest,
    WebSearchResponse,
};
