//! Sister MCP integration — connection management and cognitive dispatch.

pub mod connection;
pub mod cognitive;
pub mod cognitive_prompt;
pub mod cognitive_prompt_sections;
pub mod delegation;
pub mod learn;
pub mod perceive;

// Phase 5.5 — Deep Sister Integration
pub mod memory_deep;
pub mod contract_deep;
pub mod contract_core;
pub mod contract_extended;
pub mod contract_workspace;
pub mod contract_visibility;
pub mod contract_generation;
pub mod contract_governance;
pub mod contract_resilience;
pub mod planning_deep;
pub mod reality_deep;
pub mod aegis_deep;
pub mod comm_deep;
pub mod extras_deep;

// Phase G — Full Sister Exploitation
pub mod identity_deep;
pub mod identity_core;
pub mod identity_accountability;
pub mod identity_federation;
pub mod identity_resilience;
pub mod identity_workspace;
pub mod identity_continuity;
pub mod forge_deep;
pub mod veritas_aegis_deep;
pub mod time_deep;
pub mod evolve_deep;

// Browser Agent — multi-step web browsing pipeline
pub mod browser_agent;

// Agent Pipelines — multi-step domain pipelines
pub mod code_agent;
pub mod comm_agent;
pub mod planning_agent;

// Extended tool coverage
pub mod memory_extended;
pub mod cognition_extended;
pub mod cognition_core;
pub mod reality_extended;

// Time sister — full tool coverage
pub mod time_exploration;
pub mod time_protection;
pub mod time_management;

// Codebase sister — full 73-tool integration
pub mod codebase_deep;
pub mod codebase_extended;
pub mod codebase_omniscience;
pub mod codebase_facades;

// Memory sister — full 161-tool integration
pub mod memory_infinite;
pub mod memory_prophetic;
pub mod memory_collective;
pub mod memory_resurrection;
pub mod memory_metamemory;
pub mod memory_transcendent;
pub mod memory_v3;
pub mod memory_workspace;
pub mod memory_facades;

// Vision sister — full 112-tool integration
pub mod vision_grounding;
pub mod vision_temporal;
pub mod vision_prediction;
pub mod vision_cognition;
pub mod vision_synthesis;
pub mod vision_forensics;
pub mod vision_workspace;
pub mod vision_grammar_ext;

// Tool dispatch — <hydra-tool> tag parsing and MCP routing
pub mod tool_dispatch;

// SisterGateway — enforces sister-first pattern for all operations
pub mod gateway;
pub mod gateway_helpers;
mod gateway_tests;

pub use cognitive::{init_sisters, Sisters, SistersHandle};
pub use connection::extract_text;
pub use gateway::SisterGateway;

/// LLM micro-call for cheap classification/generation tasks.
/// Uses Haiku by default, Sonnet for judges, Opus for principles.
pub async fn llm_micro_call(
    config: &hydra_model::llm_config::LlmConfig,
    prompt: &str,
    tier: &str,
) -> Option<String> {
    let model = match tier {
        "haiku" => "claude-haiku-4-5-20251001",
        "opus" => "claude-opus-4-6",
        _ => "claude-sonnet-4-6",
    };

    let client = hydra_model::providers::anthropic::AnthropicClient::new(config).ok()?;

    let request = hydra_model::providers::CompletionRequest {
        model: model.to_string(),
        messages: vec![hydra_model::providers::Message {
            role: "user".into(),
            content: prompt.to_string(),
        }],
        max_tokens: 500,
        temperature: Some(0.3),
        system: None,
    };

    match client.complete(request).await {
        Ok(resp) => Some(resp.content),
        Err(e) => {
            eprintln!("[hydra:llm_micro] {} call failed: {:?}", tier, e);
            None
        }
    }
}
