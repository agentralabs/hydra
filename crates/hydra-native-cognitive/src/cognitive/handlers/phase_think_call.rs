//! THINK phase — LLM call execution and result handling.
//!
//! Extracted from phase_think.rs for compilation performance.
//! Builds the completion request, dispatches to the selected provider,
//! handles timeouts, and assembles the ThinkResult.

use tokio::sync::mpsc;

use hydra_native_state::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use hydra_native_state::utils::safe_truncate;

use super::super::loop_runner::CognitiveUpdate;
use super::super::intent_router::ClassifiedIntent;
use super::llm_helpers::adaptive_max_tokens;
use super::phase_think::ThinkResult;

/// Execute the LLM call and assemble the ThinkResult.
///
/// Called from `run_think` after prompt building and model selection are done.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute_llm_call(
    text: &str,
    system_prompt: String,
    api_messages: Vec<hydra_model::providers::Message>,
    active_model: &str,
    provider: &str,
    has_key: bool,
    is_simple: bool,
    is_complex: bool,
    is_action_request: bool,
    complexity: &str,
    intent: &ClassifiedIntent,
    llm_config: &hydra_model::LlmConfig,
    config_model: &str,
    sisters_handle: &Option<crate::sisters::SistersHandle>,
    perceive_ms: u64,
    think_start: std::time::Instant,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> ThinkResult {
    let prompt_est = (system_prompt.len() + 3) / 4;
    let history_est: usize = api_messages.iter().map(|m| (m.content.len() + 3) / 4).sum();
    let total_est = prompt_est + history_est;
    eprintln!("[hydra:tokens] prompt=~{} history=~{} total=~{} mode={} provider={} model={}",
        prompt_est, history_est, total_est,
        if is_simple { "simple" } else { "complex" },
        provider, active_model);
    eprintln!("[hydra:llm] SENDING to {} — {} messages, ~{} tokens total", provider, api_messages.len(), total_est);

    // Check if this task should use agentic session
    let use_agentic = hydra_kernel::orchestration::should_use_agentic_session(text);
    if use_agentic {
        eprintln!("[hydra:orchestration] Task routed to agentic session (multi-turn)");
        let _ = tx.send(CognitiveUpdate::Phase("Agentic Session".into()));
    }

    let llm_timeout = std::time::Duration::from_secs(if use_agentic { 180 } else { 90 });

    let mut agentic_session = if use_agentic {
        let session_config = hydra_kernel::orchestration::SessionConfig {
            max_turns: 10,
            turn_timeout_secs: 30,
            total_budget_tokens: 40_000,
            temperature: 0.3,
            system_prompt: Some(system_prompt.clone()),
        };
        let mut s = hydra_kernel::orchestration::AgenticSession::new(session_config);
        s.start();
        s.add_turn(hydra_kernel::orchestration::TurnRole::User, text.to_string(), 0);
        Some(s)
    } else {
        None
    };

    let llm_result = if has_key {
        let request = hydra_model::CompletionRequest {
            model: active_model.to_string(),
            messages: api_messages,
            max_tokens: {
                let model_max = match active_model {
                    m if m.contains("opus") => 32_768,
                    m if m.contains("sonnet") => 16_384,
                    m if m.contains("haiku") => 8_192,
                    m if m.contains("gpt-4o") => 16_384,
                    m if m.contains("gpt-4") => 8_192,
                    m if m.contains("gemini") => 8_192,
                    m if m.contains("deepseek") => 8_000,
                    m if m.contains("ollama") | m.contains("llama") | m.contains("phi") | m.contains("mistral") => 4_096,
                    _ => 16_384,
                };
                let adaptive_max = adaptive_max_tokens(intent, complexity, is_action_request);
                let task_max = std::cmp::min(adaptive_max, model_max);
                if is_complex { task_max } else { std::cmp::min(task_max, model_max) }
            },
            temperature: Some(if is_complex { 0.3 } else { 0.7 }),
            system: Some(system_prompt),
        };

        let llm_config = llm_config.clone();
        let active_model_owned = active_model.to_string();
        let provider_owned = provider.to_string();
        let tx_stream = tx.clone();
        let llm_future = async {
            let chunk_cb = |chunk: &str| {
                let _ = tx_stream.send(CognitiveUpdate::StreamChunk { content: chunk.to_string() });
            };
            let result = match provider_owned.as_str() {
                "anthropic" => {
                    match hydra_model::providers::anthropic::AnthropicClient::new(&llm_config) {
                        Ok(client) => client.complete_streaming(request, chunk_cb).await
                            .map(|r| (r.content, r.model, r.input_tokens, r.output_tokens))
                            .map_err(|e| format!("{}", e)),
                        Err(e) => Err(format!("{}", e)),
                    }
                }
                "openai" | "google" => {
                    match hydra_model::providers::openai::OpenAiClient::new(&llm_config) {
                        Ok(client) => client.complete_streaming(request, chunk_cb).await
                            .map(|r| (r.content, r.model, r.input_tokens, r.output_tokens))
                            .map_err(|e| format!("{}", e)),
                        Err(e) => Err(format!("{}", e)),
                    }
                }
                "ollama" => {
                    let mut ollama_config = llm_config.clone();
                    ollama_config.openai_api_key = Some("ollama".into());
                    ollama_config.openai_base_url = "http://localhost:11434".into();
                    match hydra_model::providers::openai::OpenAiClient::new(&ollama_config) {
                        Ok(client) => client.complete_streaming(request, chunk_cb).await
                            .map(|r| (r.content, r.model, r.input_tokens, r.output_tokens))
                            .map_err(|e| format!("{}", e)),
                        Err(e) => Err(format!("{}", e)),
                    }
                }
                _ => Err("Unsupported provider".into()),
            };
            let _ = tx_stream.send(CognitiveUpdate::StreamComplete);
            result
        };

        match tokio::time::timeout(llm_timeout, llm_future).await {
            Ok(result) => result,
            Err(_) => {
                eprintln!("[hydra:llm] TIMEOUT after {}s — provider={} model={}", llm_timeout.as_secs(), provider, active_model_owned);
                let _ = tx.send(CognitiveUpdate::StreamComplete);
                Err(format!("LLM request timed out after {}s. The {} API may be slow or unreachable.", llm_timeout.as_secs(), provider))
            }
        }
    } else {
        Err("No API key configured. Add your key in Settings → API Key.".into())
    };

    let think_ms = think_start.elapsed().as_millis() as u64;
    let llm_ok = llm_result.is_ok();
    match &llm_result {
        Ok((_, model, inp, out)) => eprintln!("[hydra:llm] RECEIVED in {}ms — model={} input={} output={}", think_ms, model, inp, out),
        Err(e) => eprintln!("[hydra:llm] FAILED in {}ms — error={}", think_ms, safe_truncate(e, 200)),
    }
    let (response_text, _actual_model, input_tokens, output_tokens) = match &llm_result {
        Ok((content, model, inp, out)) => (content.clone(), model.clone(), *inp, *out),
        Err(err) => (format!("Error: {}", err), config_model.to_string(), 0u64, 0u64),
    };

    // ── Phase 4: MODEL ESCALATION — detect low-quality responses ──
    if llm_result.is_ok() {
        if let Some(decision) = super::model_escalation::check_escalation(
            &response_text, intent, complexity, active_model,
        ) {
            eprintln!(
                "[hydra:escalation] DETECTED: {} → {} (reason: {})",
                active_model, decision.target_model, decision.reason,
            );
            let _ = tx.send(CognitiveUpdate::ModelEscalated {
                from: active_model.to_string(),
                to: decision.target_model.clone(),
                reason: decision.reason.to_string(),
            });
            // Escalation is recorded — future interactions will use
            // select_initial_model() with category_success_rate to
            // proactively pick stronger models for this category.
        }
    }

    // Record LLM turn in agentic session
    if let Some(ref mut session) = agentic_session {
        let turn_tokens = (input_tokens + output_tokens) as u32;
        session.add_turn(
            hydra_kernel::orchestration::TurnRole::Assistant,
            response_text.clone(),
            turn_tokens,
        );
        eprintln!(
            "[hydra:orchestration] Agentic session: {} turns, {} tokens used",
            session.turns_completed(),
            session.tokens_used()
        );
    }

    // Report token usage
    let _ = tx.send(CognitiveUpdate::TokenUsage { input_tokens, output_tokens });

    // Report which sisters were called
    if let Some(ref sh) = sisters_handle {
        let called_sisters = sh.connected_sisters_list();
        let _ = tx.send(CognitiveUpdate::SistersCalled { sisters: called_sisters });
    }

    if !is_simple {
        let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: 1, duration_ms: Some(think_ms) });
        let _ = tx.send(CognitiveUpdate::PlanStepStart(2));
    }
    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(perceive_ms) },
        PhaseStatus { phase: CognitivePhase::Think, state: PhaseState::Completed, tokens_used: Some(input_tokens + output_tokens), duration_ms: Some(think_ms) },
        PhaseStatus { phase: CognitivePhase::Decide, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));

    ThinkResult {
        response_text,
        active_model: active_model.to_string(),
        provider: provider.to_string(),
        input_tokens,
        output_tokens,
        think_ms,
        llm_config: llm_config.clone(),
        llm_ok,
    }
}
