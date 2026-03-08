use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use hydra_core::error::HydraError;
use hydra_core::types::{CognitivePhase, RiskAssessment};
use hydra_db::{RunStatus, StepRow, StepStatus};
use hydra_kernel::cognitive_loop::{CycleInput, CycleOutput, CycleStatus, PhaseHandler};
use hydra_kernel::{CognitiveLoop, KernelConfig};
use hydra_ledger::receipt::LedgerReceiptType;
use hydra_model::LlmConfig;
use hydra_runtime::cognitive::LlmPhaseHandler;
use hydra_runtime::sse::{SseEvent, SseEventType};

use crate::state::AppState;

/// Phase names used in SSE events and DB steps
const PHASE_NAMES: &[(&str, &str)] = &[
    ("perceive", "Perceiving intent"),
    ("think", "Analyzing approach"),
    ("decide", "Planning actions"),
    ("act", "Executing plan"),
    ("learn", "Recording outcomes"),
];

/// Execute a cognitive loop for a run, emitting SSE events and updating DB.
/// This is spawned as an async task from handle_run.
pub async fn execute_run(state: Arc<AppState>, run_id: String, intent: String) {
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

/// Wraps LlmPhaseHandler to emit SSE events at phase boundaries
pub struct EventEmittingHandler {
    pub inner: LlmPhaseHandler,
    state: Arc<AppState>,
    run_id: String,
    step_ids: parking_lot::Mutex<Vec<String>>,
}

impl EventEmittingHandler {
    pub fn new(inner: LlmPhaseHandler, state: Arc<AppState>, run_id: String) -> Self {
        Self {
            inner,
            state,
            run_id,
            step_ids: parking_lot::Mutex::new(Vec::new()),
        }
    }

    pub fn set_step_ids(&self, ids: Vec<String>) {
        *self.step_ids.lock() = ids;
    }

    fn emit_phase_started(&self, phase: &str, phase_index: usize) {
        let step_id = self
            .step_ids
            .lock()
            .get(phase_index)
            .cloned()
            .unwrap_or_default();

        // Update DB step to running
        let started_at = Utc::now().to_rfc3339();
        let _ = self.state.db.update_step_status(
            &step_id,
            StepStatus::Running,
            Some(&started_at),
            None,
        );

        self.state.event_bus.publish(SseEvent::new(
            SseEventType::StepStarted,
            serde_json::json!({
                "run_id": self.run_id,
                "step_id": step_id,
                "phase": phase,
                "sequence": phase_index + 1,
                "description": PHASE_NAMES[phase_index].1,
            }),
        ));
    }

    fn emit_phase_completed(
        &self,
        phase: &str,
        phase_index: usize,
        tokens: u64,
        duration_ms: u64,
        data: &serde_json::Value,
    ) {
        let step_id = self
            .step_ids
            .lock()
            .get(phase_index)
            .cloned()
            .unwrap_or_default();

        // Update DB step to completed
        let now = Utc::now().to_rfc3339();
        let _ = self.state.db.update_step_status(
            &step_id,
            StepStatus::Completed,
            Some(&now),
            Some("completed"),
        );

        self.state.event_bus.publish(SseEvent::new(
            SseEventType::StepCompleted,
            serde_json::json!({
                "run_id": self.run_id,
                "step_id": step_id,
                "phase": phase,
                "tokens_used": tokens,
                "duration_ms": duration_ms,
                "result": data,
            }),
        ));
    }
}

#[async_trait]
impl PhaseHandler for EventEmittingHandler {
    async fn perceive(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError> {
        self.emit_phase_started("perceive", 0);
        let start = Instant::now();

        let result = self.inner.perceive(input).await?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let tokens = self
            .inner
            .phase_metrics()
            .last()
            .map(|(_, t, _)| *t)
            .unwrap_or(0);
        self.emit_phase_completed("perceive", 0, tokens, duration_ms, &result);

        Ok(result)
    }

    async fn think(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.emit_phase_started("think", 1);
        let start = Instant::now();

        let result = self.inner.think(perceived).await?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let metrics = self.inner.phase_metrics();
        let tokens = metrics
            .iter()
            .filter(|(p, _, _)| *p == CognitivePhase::Think)
            .last()
            .map(|(_, t, _)| *t)
            .unwrap_or(0);
        self.emit_phase_completed("think", 1, tokens, duration_ms, &result);

        Ok(result)
    }

    async fn decide(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.emit_phase_started("decide", 2);
        let start = Instant::now();

        let result = self.inner.decide(thought).await?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let metrics = self.inner.phase_metrics();
        let tokens = metrics
            .iter()
            .filter(|(p, _, _)| *p == CognitivePhase::Decide)
            .last()
            .map(|(_, t, _)| *t)
            .unwrap_or(0);
        self.emit_phase_completed("decide", 2, tokens, duration_ms, &result);

        Ok(result)
    }

    async fn assess_risk(
        &self,
        decision: &serde_json::Value,
    ) -> Result<RiskAssessment, HydraError> {
        self.inner.assess_risk(decision).await
    }

    async fn act(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.emit_phase_started("act", 3);
        let start = Instant::now();

        let result = self.inner.act(decision).await?;

        let duration_ms = start.elapsed().as_millis() as u64;
        self.emit_phase_completed("act", 3, 0, duration_ms, &result);

        Ok(result)
    }

    async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.emit_phase_started("learn", 4);
        let start = Instant::now();

        let learn_result = self.inner.learn(result).await?;

        let duration_ms = start.elapsed().as_millis() as u64;
        self.emit_phase_completed("learn", 4, 0, duration_ms, &learn_result);

        Ok(learn_result)
    }
}
