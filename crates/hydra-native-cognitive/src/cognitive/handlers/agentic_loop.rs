//! Agentic loop engine — multi-turn tool execution with LLM feedback.
//!
//! After the initial LLM response, if it contains `<hydra-tool>` or `<hydra-exec>` tags,
//! this module executes them, feeds results back to the LLM, and repeats until the task
//! is complete or limits are reached.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::cognitive::decide::DecideEngine;
use crate::sisters::SistersHandle;
use crate::sisters::tool_dispatch::{extract_hydra_tool_tags, strip_hydra_tool_tags};
use hydra_native_state::utils::safe_truncate;
use hydra_db::HydraDb;
use hydra_runtime::undo::UndoStack;

use super::super::loop_runner::{CognitiveUpdate, CognitiveLoopConfig};
use super::actions::{extract_inline_commands, strip_hydra_exec_tags};
use super::agentic_loop_entry::AgenticLoopConfig;
use super::agentic_loop_format::{
    format_tool_results_message, has_actionable_tags, is_task_complete, strip_done_tag,
};

/// Result of a completed agentic loop.
pub(crate) struct AgenticLoopResult {
    pub final_response: String,
    pub all_exec_results: Vec<(String, String, bool)>,
    pub total_tokens: u64,
    pub turns_completed: u8,
    pub stop_reason: &'static str,
}

/// Run the multi-turn agentic loop.
///
/// Takes the initial LLM response (which contains tool/exec tags), executes them,
/// feeds results back to the LLM, and repeats until done.
pub(crate) async fn run_agentic_loop(
    text: &str,
    system_prompt: &str,
    initial_response: &str,
    loop_config: &AgenticLoopConfig,
    llm_config: &hydra_model::LlmConfig,
    active_model: &str,
    provider: &str,
    config: &CognitiveLoopConfig,
    sisters_handle: &Option<SistersHandle>,
    decide_engine: &Arc<DecideEngine>,
    _undo_stack: &Option<Arc<parking_lot::Mutex<UndoStack>>>,
    db: &Option<Arc<HydraDb>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> AgenticLoopResult {
    let loop_start = Instant::now();
    let mut messages: Vec<hydra_model::providers::Message> = Vec::new();
    let mut total_tokens: u64 = 0;
    let mut all_exec_results: Vec<(String, String, bool)> = Vec::new();
    let mut last_response = initial_response.to_string();

    // Seed conversation with user message
    messages.push(hydra_model::providers::Message {
        role: "user".into(),
        content: text.to_string(),
    });
    messages.push(hydra_model::providers::Message {
        role: "assistant".into(),
        content: initial_response.to_string(),
    });

    for turn in 1..=loop_config.max_turns {
        // Check for actionable tags in last response
        if !has_actionable_tags(&last_response) || is_task_complete(&last_response) {
            let reason = if is_task_complete(&last_response) { "task_complete" } else { "no_more_tools" };
            return AgenticLoopResult {
                final_response: strip_done_tag(&strip_hydra_tool_tags(&strip_hydra_exec_tags(&last_response))),
                all_exec_results, total_tokens, turns_completed: turn - 1, stop_reason: reason,
            };
        }

        eprintln!("[hydra:agentic] Turn {} of {}", turn, loop_config.max_turns);
        let _ = tx.send(CognitiveUpdate::Phase(format!("Agentic Turn {}", turn)));

        // Execute <hydra-tool> tags
        let tool_results = if let Some(ref sh) = sisters_handle {
            if !extract_hydra_tool_tags(&last_response).is_empty() {
                sh.execute_tool_tags(&last_response).await
            } else { vec![] }
        } else { vec![] };

        // Execute <hydra-exec> tags through security pipeline
        let exec_results = if !extract_inline_commands(&last_response).is_empty() {
            let (_, results) = super::phase_act_exec::execute_commands(
                text, &last_response, config, llm_config,
                decide_engine, sisters_handle, _undo_stack, db, tx,
            ).await;
            results
        } else { vec![] };

        // Emit structured ToolAction events for Claude Code-style display
        for (name, output) in &tool_results {
            let summary = tool_result_summary(output);
            let _ = tx.send(CognitiveUpdate::ToolAction {
                tool: name.clone(), args: String::new(),
                result: summary, success: true,
            });
        }
        for (cmd, output, success) in &exec_results {
            let summary = if *success {
                cmd_result_summary(cmd, output)
            } else {
                format!("Failed: {}", safe_truncate(output, 120))
            };
            let _ = tx.send(CognitiveUpdate::ToolAction {
                tool: "Bash".into(), args: safe_truncate(cmd, 80).to_string(),
                result: summary, success: *success,
            });
        }

        let tool_count = tool_results.len();
        let exec_count = exec_results.len();
        all_exec_results.extend(exec_results.clone());

        let _ = tx.send(CognitiveUpdate::AgenticTurn { turn, tool_count, exec_count });
        eprintln!("[hydra:agentic] Turn {}: {} tools, {} commands", turn, tool_count, exec_count);

        // If nothing was executed, we're done
        if tool_count == 0 && exec_count == 0 {
            return AgenticLoopResult {
                final_response: strip_done_tag(&strip_hydra_tool_tags(&strip_hydra_exec_tags(&last_response))),
                all_exec_results, total_tokens, turns_completed: turn, stop_reason: "no_results",
            };
        }

        // UCU backtrack + dependency_resolver: analyze failures, inject fixes into next prompt
        let mut fix_hints: Vec<String> = Vec::new();
        for (cmd, output, success) in &exec_results {
            if !*success {
                if let Some(fix) = crate::cognitive::backtrack::suggest_fix(output) {
                    fix_hints.push(format!("Fix for `{}`: {}", safe_truncate(cmd, 40), fix));
                }
                if let Some(dep) = crate::cognitive::dependency_resolver::detect_missing_dependency(output) {
                    let res = crate::cognitive::dependency_resolver::suggest_resolution(&dep);
                    if let crate::cognitive::dependency_resolver::ResolutionAction::SuggestInstall(ref install) = res.action_taken {
                        fix_hints.push(format!("Missing {}: install with `{}`", dep.name, install));
                    }
                }
            }
        }

        // Format results as a follow-up message for the LLM, including UCU fix hints
        let mut results_msg = format_tool_results_message(&tool_results, &exec_results);
        if !fix_hints.is_empty() {
            results_msg.push_str("\n\n[Hydra analysis of failures:]\n");
            for hint in &fix_hints {
                results_msg.push_str(&format!("- {}\n", hint));
            }
        }
        messages.push(hydra_model::providers::Message {
            role: "user".into(),
            content: results_msg,
        });

        // Check token budget
        if total_tokens >= loop_config.total_budget_tokens {
            eprintln!("[hydra:agentic] Token budget exhausted ({}/{})", total_tokens, loop_config.total_budget_tokens);
            return AgenticLoopResult {
                final_response: strip_done_tag(&strip_hydra_tool_tags(&strip_hydra_exec_tags(&last_response))),
                all_exec_results, total_tokens, turns_completed: turn, stop_reason: "token_budget",
            };
        }

        // UCU token_budget: dynamic per-iteration budget instead of fixed remaining/4096
        let max_tokens = crate::cognitive::token_budget::agentic_iteration_budget(
            turn, loop_config.max_turns, loop_config.total_budget_tokens, total_tokens,
        );

        match call_llm_turn(
            system_prompt, &messages, active_model, provider, llm_config,
            max_tokens, loop_config.turn_timeout_secs, tx,
        ).await {
            Ok((response_text, tokens)) => {
                total_tokens += tokens;
                last_response = response_text.clone();
                messages.push(hydra_model::providers::Message {
                    role: "assistant".into(),
                    content: response_text,
                });
            }
            Err(e) => {
                eprintln!("[hydra:agentic] LLM call failed on turn {}: {}", turn, e);
                return AgenticLoopResult {
                    final_response: strip_done_tag(&strip_hydra_tool_tags(&strip_hydra_exec_tags(&last_response))),
                    all_exec_results, total_tokens, turns_completed: turn, stop_reason: "llm_error",
                };
            }
        }
    }

    let elapsed = loop_start.elapsed().as_millis();
    eprintln!("[hydra:agentic] Max turns reached ({} turns, {}ms, {} tokens)",
        loop_config.max_turns, elapsed, total_tokens);

    AgenticLoopResult {
        final_response: strip_done_tag(&strip_hydra_tool_tags(&strip_hydra_exec_tags(&last_response))),
        all_exec_results, total_tokens, turns_completed: loop_config.max_turns, stop_reason: "max_turns",
    }
}

/// Make a single LLM call for an agentic loop turn with streaming.
async fn call_llm_turn(
    system_prompt: &str,
    messages: &[hydra_model::providers::Message],
    model: &str,
    provider: &str,
    llm_config: &hydra_model::LlmConfig,
    max_tokens: u32,
    timeout_secs: u64,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Result<(String, u64), String> {
    let request = hydra_model::CompletionRequest {
        model: model.to_string(),
        messages: messages.to_vec(),
        max_tokens,
        temperature: Some(0.3),
        system: Some(system_prompt.to_string()),
    };

    let tx_stream = tx.clone();
    let chunk_cb = move |chunk: &str| {
        let _ = tx_stream.send(CognitiveUpdate::StreamChunk { content: chunk.to_string() });
    };

    let timeout = Duration::from_secs(timeout_secs);
    let result = match provider {
        "anthropic" => {
            let client = hydra_model::providers::anthropic::AnthropicClient::new(llm_config)
                .map_err(|e| format!("Anthropic client: {}", e))?;
            tokio::time::timeout(timeout, client.complete_streaming(request, chunk_cb)).await
        }
        "openai" | "google" => {
            let client = hydra_model::providers::openai::OpenAiClient::new(llm_config)
                .map_err(|e| format!("OpenAI client: {}", e))?;
            tokio::time::timeout(timeout, client.complete_streaming(request, chunk_cb)).await
        }
        "ollama" => {
            let mut cfg = llm_config.clone();
            cfg.openai_api_key = Some("ollama".into());
            cfg.openai_base_url = std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".to_string());
            let client = hydra_model::providers::openai::OpenAiClient::new(&cfg)
                .map_err(|e| format!("Ollama client: {}", e))?;
            tokio::time::timeout(timeout, client.complete_streaming(request, chunk_cb)).await
        }
        _ => return Err(format!("Unknown provider: {}", provider)),
    };

    match result {
        Ok(Ok(response)) => {
            let tokens = response.input_tokens + response.output_tokens;
            let _ = tx.send(CognitiveUpdate::TokenUsage {
                input_tokens: response.input_tokens,
                output_tokens: response.output_tokens,
            });
            Ok((response.content, tokens))
        }
        Ok(Err(e)) => {
            let err_str = format!("{}", e);
            let lower = err_str.to_lowercase();
            let is_rate = lower.contains("429") || lower.contains("rate limit")
                || lower.contains("too many requests") || lower.contains("overloaded");
            if is_rate {
                eprintln!("[hydra:agentic] Rate limited, retrying in 30s...");
                let _ = tx.send(CognitiveUpdate::StreamChunk {
                    content: "\n[Rate limited -- retrying in 30s...]".into(),
                });
                tokio::time::sleep(Duration::from_secs(30)).await;
                let req2 = hydra_model::CompletionRequest {
                    model: model.to_string(), messages: messages.to_vec(),
                    max_tokens, temperature: Some(0.3),
                    system: Some(system_prompt.to_string()),
                };
                let tx2 = tx.clone();
                let cb2 = move |chunk: &str| { let _ = tx2.send(CognitiveUpdate::StreamChunk { content: chunk.to_string() }); };
                let retry = match provider {
                    "anthropic" => {
                        let c = hydra_model::providers::anthropic::AnthropicClient::new(llm_config).map_err(|e| format!("{}", e))?;
                        tokio::time::timeout(Duration::from_secs(timeout_secs), c.complete_streaming(req2, cb2)).await
                    }
                    _ => {
                        let c = hydra_model::providers::openai::OpenAiClient::new(llm_config).map_err(|e| format!("{}", e))?;
                        tokio::time::timeout(Duration::from_secs(timeout_secs), c.complete_streaming(req2, cb2)).await
                    }
                };
                match retry {
                    Ok(Ok(r)) => { let t = r.input_tokens + r.output_tokens; Ok((r.content, t)) }
                    Ok(Err(e2)) => Err(format!("LLM retry failed: {}", e2)),
                    Err(_) => Err(format!("Retry timeout after {}s", timeout_secs)),
                }
            } else {
                Err(format!("LLM error: {}", e))
            }
        }
        Err(_) => Err(format!("Timeout after {}s", timeout_secs)),
    }
}

/// Summarize a tool result for Claude Code-style one-liner display.
fn tool_result_summary(output: &str) -> String {
    let trimmed = output.trim();
    if trimmed.is_empty() { return "Done".into(); }
    let first_line = trimmed.lines().next().unwrap_or(trimmed);
    let line_count = trimmed.lines().count();
    if line_count > 1 {
        format!("{} ({} lines)", safe_truncate(first_line, 60), line_count)
    } else {
        safe_truncate(first_line, 80).to_string()
    }
}

/// Summarize a command execution result.
fn cmd_result_summary(cmd: &str, output: &str) -> String {
    let line_count = output.trim().lines().count();
    if line_count == 0 { return format!("Ran `{}`", safe_truncate(cmd, 60)); }
    if line_count <= 3 {
        let first = output.trim().lines().next().unwrap_or("");
        safe_truncate(first, 80).to_string()
    } else {
        format!("Ran `{}` ({} lines)", safe_truncate(cmd, 40), line_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::agentic_loop_format::*;

    #[test]
    fn test_stop_on_done_tag() {
        let response = "All tasks complete. <hydra-done/>";
        assert!(is_task_complete(response));
        assert!(!has_actionable_tags(response));
    }

    #[test]
    fn test_continue_on_tool_tags() {
        let response = r#"Let me check. <hydra-tool name="memory_query">{"q":"x"}</hydra-tool>"#;
        assert!(has_actionable_tags(response));
        assert!(!is_task_complete(response));
    }

    #[test]
    fn test_stop_on_plain_text() {
        let response = "Here is the answer to your question.";
        assert!(!has_actionable_tags(response));
        assert!(!is_task_complete(response));
    }
}
