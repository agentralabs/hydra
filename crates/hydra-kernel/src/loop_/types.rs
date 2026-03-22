//! Shared types for the cognitive loop pipeline.

use std::collections::HashMap;

use hydra_attention::AttentionFrame;
use hydra_comprehension::ComprehendedInput;
use hydra_context::ContextFrame;
use hydra_language::LanguageAnalysis;
use hydra_noticing::NoticingSignal;

/// Everything Layer 2 produced for one input.
#[derive(Debug, Clone)]
pub struct PerceivedInput {
    pub raw: String,
    pub comprehended: ComprehendedInput,
    pub language: Option<LanguageAnalysis>,
    pub context: ContextFrame,
    pub attention: AttentionFrame,
    pub signals: Vec<NoticingSignal>,
    /// Middleware enrichments keyed by middleware name.
    pub enrichments: HashMap<String, String>,
}

/// Which processing path this input takes.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutePath {
    ZeroToken { reason: String },
    Reasoning { mode: String },
    LlmShort,
    LlmLong,
}

impl RoutePath {
    pub fn token_budget(&self) -> usize {
        match self {
            Self::ZeroToken { .. } | Self::Reasoning { .. } => 0,
            Self::LlmShort => 8_000,
            Self::LlmLong => 50_000,
        }
    }
    pub fn needs_llm(&self) -> bool {
        matches!(self, Self::LlmShort | Self::LlmLong)
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::ZeroToken { .. } => "zero-token",
            Self::Reasoning { .. } => "reasoning",
            Self::LlmShort => "llm-short",
            Self::LlmLong => "llm-long",
        }
    }
}

/// Result of one complete loop cycle.
#[derive(Debug)]
pub struct CycleResult {
    pub session_id: String,
    pub domain: String,
    pub path: String,
    pub intent_summary: String,
    pub response: String,
    pub tokens_used: usize,
    pub duration_ms: u64,
    pub success: bool,
    /// Middleware enrichments collected during this cycle.
    pub enrichments: HashMap<String, String>,
}

/// Reasoning trace — captures HOW Hydra thought, not just WHAT it concluded.
/// Stored in memory so Hydra can recognize reasoning patterns across time.
#[derive(Debug, Clone)]
pub struct ReasoningTrace {
    pub input_summary: String,
    pub route_path: String,
    pub genome_matches: Vec<String>,
    pub primitives_fired: Vec<String>,
    pub enrichments_used: Vec<String>,
    pub duration_ms: u64,
}

impl ReasoningTrace {
    pub fn to_memory_content(&self) -> String {
        format!(
            "reasoning-trace | path:{} | genome:{} | primitives:{} | {}ms",
            self.route_path,
            self.genome_matches.join(","),
            self.primitives_fired.join(","),
            self.duration_ms,
        )
    }
}

/// LLM response.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub tokens_used: usize,
    pub provider: String,
    pub model: String,
    pub duration_ms: u64,
}
