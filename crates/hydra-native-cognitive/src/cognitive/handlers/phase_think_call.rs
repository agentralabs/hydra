//! THINK phase — LLM call execution and result handling.
//!
//! Extracted from phase_think.rs for compilation performance.
//! Builds the completion request, dispatches to the selected provider,
//! handles timeouts, and assembles the ThinkResult.

use tokio::sync::mpsc;

use hydra_native_state::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use hydra_native_state::utils::{safe_truncate, strip_emojis};

use super::super::loop_runner::CognitiveUpdate;
use super::super::intent_router::ClassifiedIntent;
use super::llm_helpers::adaptive_max_tokens;
use super::phase_think::ThinkResult;
use crate::sisters::SisterGateway;

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
    gateway: &Option<std::sync::Arc<SisterGateway>>,
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
        // UCU token_budget — dynamic budget based on task context
        let model_tier = crate::cognitive::context_manager::tier_from_model(active_model);
        let token_ctx = crate::cognitive::token_budget::TaskContext {
            intent: intent.category,
            complexity: complexity.to_string(),
            is_action: is_action_request,
            history_length: api_messages.len(),
            model_tier,
            has_memory_context: true,
            iteration: 0,
            runtime_budget: 50_000,
        };
        let budget = crate::cognitive::token_budget::estimate_budget(&token_ctx);
        eprintln!("[hydra:budget] max_output={} temp={:.1} reason={}",
            budget.max_output_tokens, budget.temperature, budget.reasoning);

        let request = hydra_model::CompletionRequest {
            model: active_model.to_string(),
            messages: api_messages,
            max_tokens: budget.max_output_tokens,
            temperature: Some(budget.temperature as f64),
            system: Some(system_prompt),
        };

        let llm_config = llm_config.clone();
        let active_model_owned = active_model.to_string();
        let provider_owned = provider.to_string();
        let tx_stream = tx.clone();
        let gateway_clone = gateway.clone();
        let _ = tx.send(CognitiveUpdate::Typing(true));

        // Retry wrapper: on rate limit, wait 30s + retry (same or fallback provider)
        let llm_future = async {
            let mut last_err = String::new();
            for attempt in 0..3u8 {
                let req = request.clone();
                let prov = if attempt < 2 { provider_owned.as_str() } else {
                    // 3rd attempt: try fallback provider
                    match provider_owned.as_str() {
                        "anthropic" if llm_config.openai_api_key.is_some() => "openai",
                        "openai" if llm_config.anthropic_api_key.is_some() => "anthropic",
                        _ => provider_owned.as_str(),
                    }
                };
                let tx_s = tx_stream.clone();
                let chunk_cb = move |chunk: &str| {
                    let _ = tx_s.send(CognitiveUpdate::StreamChunk { content: strip_emojis(chunk) });
                };
                let result = call_streaming_provider(prov, req, &llm_config, chunk_cb).await;
                let _ = tx_stream.send(CognitiveUpdate::StreamComplete);
                match result {
                    Ok(r) => return Ok(r),
                    Err(e) => {
                        let is_rate = is_rate_limit_err(&e);
                        eprintln!("[hydra:llm] Attempt {} failed (rate_limit={}): {}", attempt+1, is_rate, safe_truncate(&e, 100));
                        last_err = e.clone();
                        if !is_rate || attempt == 2 { break; }
                        // Sister-first: learn rate limit via gateway (Cognition + Memory)
                        if let Some(ref gw) = gateway_clone {
                            gw.learn_from_error(
                                &format!("{} rate limited", prov),
                                &format!("Wait and retry after backoff. Error: {}", safe_truncate(&e, 100)),
                            ).await;
                        }
                        let wait = if attempt == 0 { 30 } else { 60 };
                        let _ = tx_stream.send(CognitiveUpdate::StreamChunk {
                            content: format!("\n\n[Rate limited -- retrying in {}s...]", wait),
                        });
                        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                    }
                }
            }
            Err(last_err)
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

    // ── UCU COHERENCE CHECK — quality signal that affects response ──
    let mut response_text = response_text;
    if llm_result.is_ok() {
        let score = crate::cognitive::coherence_checker::quick_coherence(&response_text, text);
        if score < 0.3 {
            eprintln!("[hydra:coherence] VERY LOW score={:.2}", score);
            response_text.push_str("\n\n*Note: This response may not fully address your question. Please clarify if needed.*");
        } else if score < 0.5 {
            eprintln!("[hydra:coherence] LOW score={:.2}", score);
        }
    }

    // ── Phase 4: ESCALATION PROTOCOL (UCU) — structured multi-level recovery ──
    if llm_result.is_ok() {
        let coherence = crate::cognitive::coherence_checker::quick_coherence(&response_text, text);
        let attempt = if coherence < 0.3 { 1 } else { 0 };
        let protocol = crate::cognitive::escalation_protocol::escalate(
            &response_text, intent, complexity, active_model, attempt,
        );
        use crate::cognitive::escalation_protocol::EscalationLevel;
        match protocol.level {
            EscalationLevel::None => {}
            EscalationLevel::RetryPrompt => {
                if let Some(ref hint) = protocol.retry_prompt_hint {
                    response_text.push_str(&format!("\n\n*[Hydra self-check: {}]*", hint));
                }
            }
            EscalationLevel::DecomposeTask if !protocol.subtasks.is_empty() => {
                response_text.push_str("\n\n**This task may need to be broken down:**\n");
                for st in &protocol.subtasks { response_text.push_str(&format!("- {}\n", st)); }
            }
            EscalationLevel::UpgradeModel => {
                if let Some(ref target) = protocol.target_model {
                    let _ = tx.send(CognitiveUpdate::ModelEscalated {
                        from: active_model.to_string(), to: target.clone(),
                        reason: protocol.reason.clone(),
                    });
                }
            }
            EscalationLevel::HumanReview => {
                response_text.push_str("\n\n*I'm having difficulty with this. Could you provide more context or break it into smaller parts?*");
            }
            _ => {}
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

/// Dispatch a streaming LLM call to the right provider.
async fn call_streaming_provider(
    provider: &str,
    request: hydra_model::CompletionRequest,
    llm_config: &hydra_model::LlmConfig,
    chunk_cb: impl Fn(&str),
) -> Result<(String, String, u64, u64), String> {
    match provider {
        "anthropic" => {
            let client = hydra_model::providers::anthropic::AnthropicClient::new(llm_config)
                .map_err(|e| format!("{}", e))?;
            client.complete_streaming(request, chunk_cb).await
                .map(|r| (r.content, r.model, r.input_tokens, r.output_tokens))
                .map_err(|e| format!("{}", e))
        }
        "openai" | "google" => {
            let client = hydra_model::providers::openai::OpenAiClient::new(llm_config)
                .map_err(|e| format!("{}", e))?;
            client.complete_streaming(request, chunk_cb).await
                .map(|r| (r.content, r.model, r.input_tokens, r.output_tokens))
                .map_err(|e| format!("{}", e))
        }
        "ollama" => {
            let mut cfg = llm_config.clone();
            cfg.openai_api_key = Some("ollama".into());
            cfg.openai_base_url = std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".to_string());
            let client = hydra_model::providers::openai::OpenAiClient::new(&cfg)
                .map_err(|e| format!("{}", e))?;
            client.complete_streaming(request, chunk_cb).await
                .map(|r| (r.content, r.model, r.input_tokens, r.output_tokens))
                .map_err(|e| format!("{}", e))
        }
        _ => Err("Unsupported provider".into()),
    }
}

/// Detect rate-limit errors from provider error strings.
fn is_rate_limit_err(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("429") || lower.contains("rate limit") || lower.contains("too many requests")
        || lower.contains("overloaded") || lower.contains("capacity")
}
