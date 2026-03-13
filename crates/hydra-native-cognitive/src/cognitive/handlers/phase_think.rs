//! THINK phase — extracted from loop_runner.rs for compilation performance.
//!
//! Builds cognitive prompt, selects model/provider, calls LLM.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::cognitive::decide::DecideEngine;
use crate::cognitive::inventions::InventionEngine;
use crate::cognitive::spawner::AgentSpawner;
use crate::sisters::SistersHandle;
use hydra_native_state::utils::safe_truncate;

use super::super::loop_runner::{CognitiveLoopConfig, CognitiveUpdate};
use super::super::intent_router::ClassifiedIntent;
use super::phase_perceive::PerceiveResult;

/// Output of the THINK phase, consumed by DECIDE/ACT/LEARN.
pub(crate) struct ThinkResult {
    pub response_text: String,
    pub active_model: String,
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub think_ms: u64,
    pub llm_config: hydra_model::LlmConfig,
    pub llm_ok: bool,
}

/// Run the THINK phase: build prompt, select model, call LLM.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_think(
    text: &str,
    config: &CognitiveLoopConfig,
    intent: &ClassifiedIntent,
    perceive: &PerceiveResult,
    is_simple: bool,
    is_complex: bool,
    is_action_request: bool,
    complexity: &str,
    risk_level: &str,
    sisters_handle: &Option<SistersHandle>,
    decide_engine: &Arc<DecideEngine>,
    inventions: &Option<Arc<InventionEngine>>,
    spawner: &Option<Arc<AgentSpawner>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> ThinkResult {
    use std::time::Instant;

    let _ = tx.send(CognitiveUpdate::Phase("Think".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));
    let think_start = Instant::now();

    // Cognition sister: predict intent and detect drift
    if let Some(ref sh) = sisters_handle {
        if let Some(prediction) = sh.cognition_predict_intent(text).await {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Cognition Prediction".into(), content: prediction,
            });
        }
        if let Some(drift) = sh.cognition_detect_drift().await {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Behavioral Drift".into(), content: drift,
            });
        }
    }

    // Sub-agent spawning for complex tasks
    if let Some(ref spawner) = spawner {
        if spawner.should_spawn(text) {
            let subtasks = spawner.decompose(text);
            let session_id = spawner.create_session(text, &subtasks);
            let _ = tx.send(CognitiveUpdate::Phase(format!(
                "Spawning {} sub-agents for parallel execution",
                subtasks.len()
            )));
            for st in &subtasks {
                let _ = tx.send(CognitiveUpdate::PlanStepStart(0));
                eprintln!("[hydra] Sub-agent {}: {}", st.module, st.description);
            }
            spawner.complete_session(&session_id);
        }
    }

    // Forge blueprinting (complex only)
    let forge_blueprint = if is_complex {
        if let Some(ref sh) = sisters_handle {
            let _ = tx.send(CognitiveUpdate::Phase("Think (Forge blueprint)".into()));
            sh.think_forge(text).await
        } else { None }
    } else { None };

    // Veritas intent compilation (complex only)
    let veritas_intent = if is_complex {
        if let Some(ref sh) = sisters_handle {
            sh.think_veritas(text).await
        } else { None }
    } else { None };

    // Build LLM config with provider auto-fallback (sanitized keys)
    let mut llm_config = hydra_model::LlmConfig::from_env_with_overlay(
        &config.anthropic_key,
        &config.openai_key,
        config.anthropic_oauth_token.as_deref(),
    );

    let mut active_model = config.model.clone();
    let mut provider = if active_model.contains("claude") {
        "anthropic"
    } else if active_model.contains("gpt") {
        "openai"
    } else if active_model.contains("gemini") {
        "google"
    } else if active_model == "ollama" {
        "ollama"
    } else {
        "anthropic"
    };

    // Probe LLM capabilities
    let llm_profile = hydra_router::LLMCapabilityProfile::from_model_id(&active_model, provider);
    eprintln!(
        "[hydra:probe] Model={} provider={} tools={} parallel={} vision={} ctx={}k format={:?}",
        llm_profile.model_id, llm_profile.provider,
        llm_profile.supports_tool_use, llm_profile.supports_parallel_tools,
        llm_profile.supports_vision, llm_profile.max_context_tokens / 1000,
        llm_profile.native_tool_format,
    );

    // Dynamic per-phase model selection
    // Phase 4: Use model escalation's select_initial_model for baseline,
    // then apply context-aware overrides below.
    let has_memory_context = perceive.always_on_memory.is_some();
    if provider == "anthropic" && !active_model.contains("opus") {
        let escalation_model = super::model_escalation::select_initial_model(
            &intent, complexity, None, // category_success_rate passed via caller in future
        );
        if escalation_model != active_model && !active_model.contains("opus") {
            eprintln!(
                "[hydra:escalation] Initial model selection: {} (was {})",
                escalation_model, active_model,
            );
            active_model = escalation_model.to_string();
        }
    }
    if provider == "anthropic" {
        use super::super::intent_router::IntentCategory as IC;
        let routed_model: Option<&str> = match intent.category {
            // Greetings/farewell/thanks are handled by dispatch_intents (static responses).
            // If they reach here, they have memory context — use Sonnet.
            IC::Greeting | IC::Farewell | IC::Thanks if has_memory_context => {
                Some("claude-sonnet-4-6")
            }
            IC::Greeting | IC::Farewell | IC::Thanks => {
                Some("claude-haiku-4-5-20251001")
            }
            // Memory recall and questions with memory need Sonnet for natural response
            IC::MemoryRecall | IC::Question if has_memory_context && !active_model.contains("opus") => {
                Some("claude-sonnet-4-6")
            }
            IC::CodeBuild | IC::CodeFix
                if complexity == "complex" && !active_model.contains("opus") => {
                Some("claude-sonnet-4-6")
            }
            _ => match complexity {
                "simple" if !active_model.contains("opus") && !has_memory_context => {
                    Some("claude-haiku-4-5-20251001")
                }
                "simple" if !active_model.contains("opus") => {
                    Some("claude-sonnet-4-6") // has memory → needs personality
                }
                "complex" if !active_model.contains("opus") => {
                    Some("claude-sonnet-4-6")
                }
                _ => None,
            },
        };
        if let Some(routed) = routed_model {
            if routed != active_model {
                eprintln!("[hydra:routing] Model routed: {} → {} (intent={:?}, complexity={})", active_model, routed, intent.category, complexity);
                active_model = routed.to_string();
            }
        }
    } else if provider == "openai" {
        use super::super::intent_router::IntentCategory as IC2;
        match intent.category {
            IC2::Greeting | IC2::Farewell | IC2::Thanks
                if active_model != "gpt-4o-mini" => {
                eprintln!("[hydra:routing] Model routed: {} → gpt-4o-mini (greeting)", active_model);
                active_model = "gpt-4o-mini".to_string();
            }
            _ => match complexity {
                "simple" if active_model != "gpt-4o-mini" => {
                    eprintln!("[hydra:routing] Model routed: {} → gpt-4o-mini (simple)", active_model);
                    active_model = "gpt-4o-mini".to_string();
                }
                "complex" if active_model != "gpt-4o" => {
                    eprintln!("[hydra:routing] Model routed: {} → gpt-4o (complex)", active_model);
                    active_model = "gpt-4o".to_string();
                }
                _ => {}
            },
        }
    }

    // Auto-fallback: if selected provider has no key, switch
    let provider_has_key = match provider {
        "anthropic" => llm_config.anthropic_api_key.is_some(),
        "openai" => llm_config.openai_api_key.is_some(),
        "google" => !config.google_key.is_empty(),
        "ollama" => true,
        _ => false,
    };
    if !provider_has_key {
        if llm_config.openai_api_key.is_some() {
            provider = "openai";
            active_model = "gpt-4o".to_string();
        } else if llm_config.anthropic_api_key.is_some() {
            provider = "anthropic";
            active_model = "claude-sonnet-4-6".to_string();
        } else if !config.google_key.is_empty() {
            provider = "google";
            active_model = "gemini-2.0-flash".to_string();
        }
    }

    if provider == "google" && !config.google_key.is_empty() {
        llm_config.openai_api_key = Some(config.google_key.clone());
        llm_config.openai_base_url = "https://generativelanguage.googleapis.com".into();
    }

    let has_key = match provider {
        "anthropic" => llm_config.anthropic_api_key.is_some(),
        "openai" => llm_config.openai_api_key.is_some(),
        "google" => !config.google_key.is_empty(),
        "ollama" => true,
        _ => false,
    };

    // Build COGNITIVE system prompt
    let system_prompt = super::phase_think_prompt::build_system_prompt(
        text, config, intent, perceive, is_simple, is_complex, is_action_request,
        complexity, sisters_handle, decide_engine, inventions, &forge_blueprint,
        &veritas_intent, &active_model,
    );

    // Intelligent contextual compression
    let system_prompt = if let Some(ref inv) = inventions {
        let est_tokens = (system_prompt.len() + 3) / 4;
        if est_tokens > 4000 {
            let (compressed, ratio) = inv.compress_context(&system_prompt);
            let compressed_est = (compressed.len() + 3) / 4;
            if ratio > 0.05 {
                eprintln!("[hydra:compress] Saved ~{} tokens ({:.0}% reduction)",
                    est_tokens - compressed_est, ratio * 100.0);
                let _ = tx.send(CognitiveUpdate::CompressionApplied {
                    original_tokens: est_tokens,
                    compressed_tokens: compressed_est,
                    ratio,
                });
            }
            compressed
        } else {
            system_prompt
        }
    } else {
        system_prompt
    };

    // Build messages with conversation history
    let history_limit = if is_simple { 6 } else { 20 };
    let history_start = config.history.len().saturating_sub(history_limit);
    let max_msg_chars = if is_simple { 500 } else { 2000 };
    let mut api_messages: Vec<hydra_model::providers::Message> = Vec::new();
    for (role, content) in &config.history[history_start..] {
        let trimmed = if content.len() > max_msg_chars {
            format!("{}...", safe_truncate(content, max_msg_chars))
        } else {
            content.clone()
        };
        api_messages.push(hydra_model::providers::Message {
            role: role.clone(),
            content: trimmed,
        });
    }

    // Delegate LLM call execution to phase_think_call
    super::phase_think_call::execute_llm_call(
        text,
        system_prompt,
        api_messages,
        &active_model,
        provider,
        has_key,
        is_simple,
        is_complex,
        is_action_request,
        complexity,
        intent,
        &llm_config,
        &config.model,
        sisters_handle,
        perceive.perceive_ms,
        think_start,
        tx,
    ).await
}
