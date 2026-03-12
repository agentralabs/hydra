//! ACT phase — extracted from loop_runner.rs for compilation performance.
//!
//! Executes the plan through sisters, runs commands with full security pipeline,
//! handles vision capture, self-review, receipts, and failure belief generation.
//!
//! Command execution pipeline lives in `phase_act_exec`.

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::cognitive::decide::DecideEngine;
use crate::sisters::SistersHandle;
use hydra_native_state::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use hydra_native_state::utils::{extract_json_plan, format_bytes, safe_truncate};
use hydra_db::{HydraDb, BeliefRow};
use hydra_runtime::undo::UndoStack;

use super::super::loop_runner::CognitiveUpdate;
use super::execution::{execute_json_plan, maybe_deepen_project};
use super::llm_helpers::self_review_response;
use super::memory::md5_simple;
use super::phase_act_exec::execute_commands;

/// Output of the ACT phase, consumed by LEARN.
pub(crate) struct ActResult {
    pub final_response: String,
    pub all_exec_results: Vec<(String, String, bool)>,
    pub act_ms: u64,
}

/// Run the ACT phase: execute plan, run commands, capture vision, sign receipts.
pub(crate) async fn run_act(
    text: &str,
    config: &super::super::loop_runner::CognitiveLoopConfig,
    response_text: &str,
    is_simple: bool,
    is_complex: bool,
    llm_ok: bool,
    llm_config: &hydra_model::LlmConfig,
    provider: &str,
    active_model: &str,
    risk_level: &str,
    gate_decision: &str,
    decide_engine: &Arc<DecideEngine>,
    sisters_handle: &Option<SistersHandle>,
    undo_stack: &Option<Arc<parking_lot::Mutex<UndoStack>>>,
    db: &Option<Arc<HydraDb>>,
    input_tokens: u64,
    output_tokens: u64,
    perceive_ms: u64,
    think_ms: u64,
    decide_ms: u64,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> ActResult {
    let _ = tx.send(CognitiveUpdate::Phase("Act".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));
    let act_start = Instant::now();

    // Alias for compatibility
    let llm_result: Result<(), String> = if llm_ok { Ok(()) } else { Err("LLM failed".into()) };

    let mut final_response = response_text.to_string();
    if is_complex && llm_result.is_ok() {
        let json_plan = extract_json_plan(response_text);
        if let Some(ref plan) = json_plan {
            final_response = execute_json_plan(plan, tx, undo_stack).await;

            // Multi-pass deepening: if generated files are shallow stubs, expand them
            let home = std::env::var("HOME").unwrap_or_default();
            let project_dir_name = plan["project_dir"].as_str().unwrap_or("hydra-project");
            let base_dir = format!("{}/projects/{}", home, project_dir_name);
            let summary = plan["summary"].as_str().unwrap_or("Project");
            if let Some(updated) = maybe_deepen_project(
                &base_dir,
                summary,
                llm_config,
                provider,
                active_model,
                tx,
            ).await {
                // Append deepening metrics to the response
                final_response.push_str(&format!(
                    "\n\n### Multi-Pass Deepening\n\
                     | Metric | Value |\n\
                     |--------|-------|\n\
                     | Modules deepened | **{}** |\n\
                     | Files expanded | **{}** |\n\
                     | New total lines | **{}** |\n\
                     | New total size | **{}** |\n",
                    updated.modules_deepened,
                    updated.files_expanded,
                    updated.total_lines,
                    format_bytes(updated.total_bytes),
                ));
            }
        }
    }

    // Phase 2, A1: Track exec results for failure belief generation
    let mut all_exec_results: Vec<(String, String, bool)> = Vec::new();

    // ── Inline command execution ──
    // Two strategies:
    // 1. Parse <hydra-exec> tags if the LLM included them
    // 2. Detect action intent from the user's message and execute directly
    // EVERY command goes through the execution gate for risk evaluation.
    if llm_result.is_ok() {
        let (updated, exec_results) = execute_commands(
            text,
            &final_response,
            config,
            llm_config,
            decide_engine,
            sisters_handle,
            undo_stack,
            db,
            tx,
        ).await;
        final_response = updated;

        // Phase 2, A1: Copy exec results out for failure belief generation
        all_exec_results = exec_results;
    }

    // Phase 2, X2: Self-Review Before Delivery
    // For complex queries only, do a quick validation of the response before delivering.
    if is_complex && llm_result.is_ok() {
        let review_result = self_review_response(text, &final_response, llm_config).await;
        if let Some(issue) = review_result {
            eprintln!("[hydra:review] Self-review flagged issue: {}", issue);
            final_response.push_str(&format!(
                "\n\n---\n*Note: {}*",
                issue
            ));
        }
    }

    // Sign receipt via Identity sister
    if let Some(ref sh) = sisters_handle {
        if let Some(id) = &sh.identity {
            let _ = id.call_tool("receipt_create", serde_json::json!({
                "action": text,
                "risk_level": risk_level,
                "gate_decision": gate_decision,
                "tokens_used": input_tokens + output_tokens,
            })).await;
        }
    }

    // Record trust outcome — success earns trust, failure loses it
    if llm_result.is_ok() {
        decide_engine.record_success(risk_level, "");
    } else {
        decide_engine.record_failure(risk_level, "");
    }

    // Phase 2, A1: Failure Belief Generation
    // When commands fail, create beliefs so future interactions can avoid the same mistakes
    if let Some(ref db) = db {
        for (cmd, output, success) in &all_exec_results {
            if !*success {
                let now = chrono::Utc::now().to_rfc3339();
                let subject = safe_truncate(cmd, 60).to_string();
                let failure_id = format!("fail-{}", md5_simple(&format!("{}:{}", cmd, output)));
                let content = format!("Command `{}` failed: {}", safe_truncate(cmd, 100), safe_truncate(output, 200));
                let _ = db.upsert_belief(&BeliefRow {
                    id: failure_id,
                    category: "failure_pattern".to_string(),
                    subject,
                    content,
                    confidence: 0.9,
                    source: "execution_failure".to_string(),
                    confirmations: 0,
                    contradictions: 0,
                    active: true,
                    supersedes: None,
                    superseded_by: None,
                    created_at: now.clone(),
                    updated_at: now,
                });
            }
        }
    }

    let act_ms = act_start.elapsed().as_millis() as u64;

    if !is_simple {
        let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: 3, duration_ms: Some(act_ms) });
    }
    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(perceive_ms) },
        PhaseStatus { phase: CognitivePhase::Think, state: PhaseState::Completed, tokens_used: Some(input_tokens + output_tokens), duration_ms: Some(think_ms) },
        PhaseStatus { phase: CognitivePhase::Decide, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(decide_ms) },
        PhaseStatus { phase: CognitivePhase::Act, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(act_ms) },
        PhaseStatus { phase: CognitivePhase::Learn, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));

    ActResult {
        final_response,
        all_exec_results,
        act_ms,
    }
}
