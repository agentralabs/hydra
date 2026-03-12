use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use chrono::Utc;

use hydra_core::error::HydraError;
use hydra_core::types::{CognitivePhase, RiskAssessment};
use hydra_db::StepStatus;
use hydra_kernel::cognitive_loop::{CycleInput, PhaseHandler};
use hydra_runtime::cognitive::LlmPhaseHandler;
use hydra_runtime::sse::{SseEvent, SseEventType};

use crate::state::AppState;

use super::PHASE_NAMES;

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

    fn emit_phase_progress(&self, phase: &str, phase_index: usize, detail: &str) {
        let step_id = self
            .step_ids
            .lock()
            .get(phase_index)
            .cloned()
            .unwrap_or_default();

        self.state.event_bus.publish(SseEvent::new(
            SseEventType::StepProgress,
            serde_json::json!({
                "run_id": self.run_id,
                "step_id": step_id,
                "phase": phase,
                "detail": detail,
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
        self.emit_phase_progress("perceive", 0, "Parsing intent and gathering context");

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
        self.emit_phase_progress("think", 1, "Reasoning about approach and constraints");

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
        self.emit_phase_progress("decide", 2, "Selecting actions and assessing risk");

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
        self.emit_phase_progress("act", 3, "Executing planned actions");

        let result = self.inner.act(decision).await?;

        let duration_ms = start.elapsed().as_millis() as u64;
        self.emit_phase_completed("act", 3, 0, duration_ms, &result);

        Ok(result)
    }

    async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.emit_phase_started("learn", 4);
        let start = Instant::now();
        self.emit_phase_progress("learn", 4, "Recording outcomes and updating knowledge");

        let learn_result = self.inner.learn(result).await?;

        let duration_ms = start.elapsed().as_millis() as u64;
        self.emit_phase_completed("learn", 4, 0, duration_ms, &learn_result);

        Ok(learn_result)
    }
}
