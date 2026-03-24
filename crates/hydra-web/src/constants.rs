//! Configurable constants for the web engine.

/// Default max results to return.
pub const DEFAULT_MAX_RESULTS: usize = 10;

/// Max pages to deep-fetch for content extraction.
pub const MAX_DEEP_FETCH_PAGES: usize = 5;

/// Cache TTL: general queries (24 hours).
pub const CACHE_TTL_GENERAL_SECS: u64 = 86_400;

/// Cache TTL: news queries (1 hour).
pub const CACHE_TTL_NEWS_SECS: u64 = 3_600;

/// Cache TTL: documentation queries (7 days).
pub const CACHE_TTL_DOCS_SECS: u64 = 604_800;

/// Per-engine timeout for search requests.
pub const ENGINE_TIMEOUT_SECS: u64 = 8;

/// Per-page timeout for deep fetch.
pub const DEEP_FETCH_TIMEOUT_SECS: u64 = 10;

/// Skip pages with less content than this.
pub const MIN_CONTENT_LENGTH: usize = 200;

/// Truncate extracted content beyond this.
pub const MAX_CONTENT_LENGTH: usize = 15_000;

/// Max chars sent to LLM for synthesis.
pub const SYNTHESIS_MAX_INPUT_CHARS: usize = 20_000;

// ── Source reliability baselines ──

pub const RELIABILITY_DDG: f64 = 0.65;
pub const RELIABILITY_WIKIPEDIA: f64 = 0.85;
pub const RELIABILITY_GITHUB: f64 = 0.75;
pub const RELIABILITY_STACKEXCHANGE: f64 = 0.78;
pub const RELIABILITY_KNOWLEDGE_INDEX: f64 = 0.90;
pub const RELIABILITY_GENOME_CACHE: f64 = 0.95;
