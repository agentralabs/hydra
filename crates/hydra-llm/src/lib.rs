//! `hydra-llm` — Standalone LLM adapter crate.
//!
//! WARNING: The kernel has its OWN production LLM adapter at
//! `hydra-kernel/src/loop_/llm.rs` (339 lines, 4 providers, retry logic).
//! This crate is a PARALLEL implementation, NOT currently wired into the
//! cognitive loop. DO NOT add this as a kernel dependency without a full
//! migration plan and harness verification (47/47 V1 + V2 behavioral scores).
//! The kernel's inline adapter is battle-tested and produces the receipts
//! that the harness validates. Swapping it could break the entire pipeline.

pub mod adapters;
pub mod config;
pub mod errors;

use async_trait::async_trait;

/// A single message in an LLM conversation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Response from an LLM provider.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

/// The core LLM adapter trait. Every provider implements this.
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    /// Send messages and get a response.
    async fn complete(&self, messages: &[Message]) -> Result<LlmResponse, errors::LlmError>;

    /// Provider name for diagnostics.
    fn provider_name(&self) -> &str;

    /// Model identifier being used.
    fn model_name(&self) -> &str;
}
