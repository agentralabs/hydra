mod handler;

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use hydra_db::{RunStatus, StepRow, StepStatus};
use hydra_kernel::cognitive_loop::{CycleInput, CycleOutput, CycleStatus};
use hydra_kernel::{CognitiveLoop, KernelConfig};
use hydra_ledger::receipt::LedgerReceiptType;
use hydra_model::LlmConfig;
use hydra_runtime::cognitive::LlmPhaseHandler;
use hydra_runtime::sse::{SseEvent, SseEventType};

use crate::state::AppState;

pub use handler::EventEmittingHandler;

/// Phase names used in SSE events and DB steps
const PHASE_NAMES: &[(&str, &str)] = &[
    ("perceive", "Perceiving intent"),
    ("think", "Analyzing approach"),
    ("decide", "Planning actions"),
    ("act", "Executing plan"),
    ("learn", "Recording outcomes"),
];

/// Execute using the FULL cognitive engine (21 modules, 17 sisters) when available.
/// Falls back to lightweight kernel if sisters aren't initialized.
pub async fn execute_run(state: Arc<AppState>, run_id: String, intent: String) {
    // If sisters are available, use the full cognitive loop (same as TUI/Desktop)
    if state.sisters.is_some() {
        execute_run_full(state, run_id, intent).await;
        return;
    }
    // Fallback: lightweight kernel (no sisters)
    execute_run_kernel(state, run_id, intent).await;
}

/// Full cognitive engine execution — all 21 modules, 17 sisters, profiles, beliefs.
async fn execute_run_full(state: Arc<AppState>, run_id: String, intent: String) {
    use hydra_native::cognitive::{CognitiveLoopConfig, CognitiveUpdate, run_cognitive_loop};
    let overlay = state.prompt_overlay.lock().clone();
    let beliefs = state.active_profile.lock().as_ref()
        .map(|p| p.beliefs.clone()).unwrap_or_default();

    let config = CognitiveLoopConfig {
        text: intent.clone(),
        anthropic_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
        openai_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        google_key: String::new(),
        model: std::env::var("HYDRA_MODEL").unwrap_or_else(|_| "claude-sonnet-4-6".into()),
        user_name: String::new(),
        task_id: run_id.clone(),
        history: Vec::new(),
        session_count: 0,
        anthropic_oauth_token: None,
        runtime: Default::default(),
        prompt_overlay: overlay,
        active_beliefs: beliefs,
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<CognitiveUpdate>();
    let sisters = state.sisters.clone();
    let decide = state.decide_engine.clone();
    let inv = Some(state.invention_engine.clone());
    let notifier = Some(state.proactive_notifier.clone());
    let spawner = Some(state.agent_spawner.clone());
    let swarm = Some(state.swarm.clone());
    let fed = Some(state.federation.clone());

    // Update DB: running
    let _ = state.db.update_run_status(&run_id, RunStatus::Running, None);
    state.event_bus.publish(SseEvent::new(SseEventType::RunStarted,
        serde_json::json!({"run_id": run_id, "intent": intent, "engine": "full"})));

    // Spawn the full cognitive loop
    let rid = run_id.clone();
    tokio::spawn(async move {
        run_cognitive_loop(config, sisters, tx, decide, None, inv, notifier, spawner,
            None, None, fed, swarm).await;
    });

    // Forward cognitive updates to SSE so TUI/clients receive streamed responses
    let mut final_response = String::new();
    while let Some(update) = rx.recv().await {
        match update {
            CognitiveUpdate::StreamChunk { content } => {
                // TUI expects: {"type":"text","content":"...","run_id":"..."}
                state.event_bus.publish(SseEvent::new(SseEventType::StreamChunk,
                    serde_json::json!({"type": "text", "run_id": run_id, "content": content})));
                final_response.push_str(&content);
            }
            CognitiveUpdate::Message { content, css_class: _, role: _ } => {
                state.event_bus.publish(SseEvent::new(SseEventType::StreamChunk,
                    serde_json::json!({"type": "text", "run_id": run_id, "content": content})));
                if final_response.is_empty() { final_response = content; }
            }
            CognitiveUpdate::Phase(phase) => {
                state.event_bus.publish(SseEvent::new(SseEventType::StreamChunk,
                    serde_json::json!({"type": "thinking", "run_id": run_id, "content": phase})));
            }
            CognitiveUpdate::ToolAction { tool, args: _, result: _, success: _ } => {
                state.event_bus.publish(SseEvent::new(SseEventType::StreamChunk,
                    serde_json::json!({"type": "tool_start", "run_id": run_id, "tool": tool})));
            }
            CognitiveUpdate::ResetIdle => break,
            _ => {}
        }
    }

    // Signal stream completion to TUI clients
    state.event_bus.publish(SseEvent::new(SseEventType::StreamChunk,
        serde_json::json!({"type": "done", "run_id": run_id, "content": final_response})));

    let now = Utc::now().to_rfc3339();
    let _ = state.db.update_run_status(&run_id, RunStatus::Completed, Some(&now));
    state.event_bus.publish(SseEvent::new(SseEventType::RunCompleted,
        serde_json::json!({"run_id": run_id, "status": "success", "response": final_response, "engine": "full"})));
}

/// Lightweight kernel execution (fallback when sisters aren't available).
async fn execute_run_kernel(state: Arc<AppState>, run_id: String, intent: String) {
    let config = KernelConfig::default();
    let kernel = CognitiveLoop::new(config);

    // Use LlmPhaseHandler for real LLM calls, with event bus for phase-level SSE
    let handler = EventEmittingHandler::new(
        LlmPhaseHandler::with_llm_config(LlmConfig::from_env()),
        state.clone(),
        run_id.clone(),
    );

    // Emit run started
    state.event_bus.publish(SseEvent::new(
        SseEventType::RunStarted,
        serde_json::json!({
            "run_id": run_id,
            "intent": intent,
            "estimated_steps": 5,
        }),
    ));

    // Update DB: running
    let _ = state
        .db
        .update_run_status(&run_id, RunStatus::Running, None);

    // Create steps for each phase
    let step_ids: Vec<String> = PHASE_NAMES
        .iter()
        .enumerate()
        .map(|(i, (_, desc))| {
            let step_id = Uuid::new_v4().to_string();
            let step = StepRow {
                id: step_id.clone(),
                run_id: run_id.clone(),
                sequence: (i + 1) as i32,
                description: desc.to_string(),
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                result: None,
                evidence_refs: None,
            };
            let _ = state.db.create_step(&step);
            step_id
        })
        .collect();

    // Store step IDs for the handler to use
    handler.set_step_ids(step_ids.clone());

    // Execute the cognitive loop with real LLM
    let input = CycleInput::simple(&intent);
    let output = kernel.run(input, &handler).await;

    // Record trust outcome on the decide engine
    if output.is_ok() {
        state.decide_engine.record_success("low", "");
    } else {
        state.decide_engine.record_failure("low", "");
    }

    // Run proactive anticipation on the intent
    {
        let mut notifier = state.proactive_notifier.lock();
        notifier.anticipate(&intent);
    }

    // Log compression stats if context is available
    if let Some(reasoning) = output.result.get("reasoning").and_then(|v| v.as_str()) {
        let (_compressed, ratio) = state.invention_engine.compress_context(reasoning);
        if ratio > 0.1 {
            tracing::debug!(compression_ratio = %ratio, "Context compression applied");
        }
    }

    // Get real token metrics from the inner LLM handler
    let total_tokens = handler.inner.total_tokens();
    let phase_metrics = handler.inner.phase_metrics();

    // Generate receipt with real token data
    let receipt = state.ledger.build_receipt(
        if output.is_ok() {
            LedgerReceiptType::ActionExecuted
        } else {
            LedgerReceiptType::ActionFailed
        },
        format!("run:{}", run_id),
        serde_json::json!({
            "run_id": run_id,
            "intent": intent,
            "status": if output.is_ok() { "completed" } else { "failed" },
            "tokens_used": total_tokens,
            "phases_completed": output.phases_completed.len(),
            "phase_metrics": phase_metrics.iter().map(|(phase, tokens, duration_ms)| {
                serde_json::json!({
                    "phase": format!("{:?}", phase),
                    "tokens": tokens,
                    "duration_ms": duration_ms,
                })
            }).collect::<Vec<_>>(),
        }),
    );
    let _ = state.ledger.record(receipt);

    // Finalize any remaining steps
    let completed_count = output.phases_completed.len();
    for (i, step_id) in step_ids.iter().enumerate() {
        if i >= completed_count {
            let now = Utc::now().to_rfc3339();
            let status = if output.is_ok() {
                StepStatus::Skipped
            } else {
                StepStatus::Failed
            };
            let _ = state
                .db
                .update_step_status(step_id, status, Some(&now), None);
        }
    }

    // Update DB: final status + emit completion event
    let now = Utc::now().to_rfc3339();
    if output.is_ok() {
        let _ = state
            .db
            .update_run_status(&run_id, RunStatus::Completed, Some(&now));

        // Extract the final response from the last phase output
        let final_response = extract_response(&output);

        // Signal stream completion to TUI clients
        state.event_bus.publish(SseEvent::new(SseEventType::StreamChunk,
            serde_json::json!({"type": "done", "run_id": run_id, "content": final_response})));

        state.event_bus.publish(SseEvent::new(
            SseEventType::RunCompleted,
            serde_json::json!({
                "run_id": run_id,
                "status": "success",
                "tokens_used": total_tokens,
                "response": final_response,
            }),
        ));
    } else {
        let _ = state
            .db
            .update_run_status(&run_id, RunStatus::Failed, Some(&now));
        let error_msg = match &output.status {
            CycleStatus::Failed(msg) => msg.clone(),
            CycleStatus::TimedOut => "Cognitive loop timed out".into(),
            CycleStatus::BudgetExceeded => "Token budget exceeded".into(),
            CycleStatus::Interrupted => "Run was interrupted".into(),
            CycleStatus::Cancelled => "Run was cancelled".into(),
            _ => "Unknown error".into(),
        };
        state.event_bus.publish(SseEvent::new(
            SseEventType::RunError,
            serde_json::json!({
                "run_id": run_id,
                "error": error_msg,
            }),
        ));
    }
}

/// Extract a user-facing response from the cognitive loop output
fn extract_response(output: &CycleOutput) -> String {
    // Try to extract meaningful content from the result JSON
    let result = &output.result;

    // Check for reasoning (from think phase)
    if let Some(reasoning) = result.get("reasoning").and_then(|v| v.as_str()) {
        if !reasoning.is_empty() {
            return reasoning.to_string();
        }
    }

    // Check for summary (from learn phase)
    if let Some(summary) = result.get("summary").and_then(|v| v.as_str()) {
        if !summary.is_empty() {
            return summary.to_string();
        }
    }

    // Check for action (from act/decide phase)
    if let Some(action) = result.get("action").and_then(|v| v.as_str()) {
        if !action.is_empty() && action != "none" {
            return format!("Action: {}", action);
        }
    }

    format!(
        "Completed {} phases. {}",
        output.phases_completed.len(),
        if output.tokens_used > 0 {
            format!("Used {} tokens.", output.tokens_used)
        } else {
            String::new()
        }
    )
}
