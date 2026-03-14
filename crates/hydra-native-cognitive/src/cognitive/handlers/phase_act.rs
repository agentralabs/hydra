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
use crate::sisters::{SistersHandle, SisterGateway};
use hydra_native_state::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use hydra_native_state::utils::{extract_json_plan, format_bytes, safe_truncate};
use hydra_db::{HydraDb, BeliefRow};
use hydra_runtime::undo::UndoStack;

use super::super::loop_runner::CognitiveUpdate;
use super::agentic_loop;
use super::agentic_loop_entry;
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
    perceive_memory: &Option<String>,
    task_plan: &Option<crate::cognitive::iterative_planner::TaskPlan>,
    decide_engine: &Arc<DecideEngine>,
    sisters_handle: &Option<SistersHandle>,
    undo_stack: &Option<Arc<parking_lot::Mutex<UndoStack>>>,
    db: &Option<Arc<HydraDb>>,
    input_tokens: u64,
    output_tokens: u64,
    perceive_ms: u64,
    think_ms: u64,
    decide_ms: u64,
    gateway: &Option<Arc<SisterGateway>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> ActResult {
    let _gateway = gateway; // Available for future sister-first risk assessment
    let _ = tx.send(CognitiveUpdate::Phase("Act".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));
    let act_start = Instant::now();
    let rt = &config.runtime;

    // Runtime permission enforcement — block actions the user has disabled
    if !rt.shell_exec && response_text.contains("<hydra-exec") {
        eprintln!("[hydra:act] Shell execution disabled by user settings");
        let _ = tx.send(CognitiveUpdate::Message { role: "hydra".into(), content: "Shell execution is disabled in your settings. Enable it in Settings > Policies to allow command execution.".into(), css_class: "message hydra".into() });
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return ActResult { final_response: response_text.to_string(), all_exec_results: vec![], act_ms: act_start.elapsed().as_millis() as u64 };
    }

    // Alias for compatibility
    let llm_result: Result<(), String> = if llm_ok { Ok(()) } else { Err("LLM failed".into()) };

    // Planning sister: create goal for complex tasks
    let planning_goal_id = if is_complex {
        if let Some(ref sh) = sisters_handle {
            sh.planning_create_goal(
                &hydra_native_state::utils::safe_truncate(text, 80),
                &[], None,
            ).await
        } else { None }
    } else { None };

    // Planning: checkpoint phase for crash recovery
    if let Some(ref goal_id) = planning_goal_id {
        if let Some(ref sh) = sisters_handle {
            sh.planning_checkpoint_phase(goal_id, "act", "started", "ACT phase beginning").await;
        }
    }

    let mut final_response = response_text.to_string();
    if is_complex && llm_result.is_ok() {
        let json_plan = extract_json_plan(response_text);
        if let Some(ref plan) = json_plan {
            final_response = execute_json_plan(plan, tx, undo_stack).await;

            // Multi-pass deepening: if generated files are shallow stubs, expand them
            let home = hydra_native_state::utils::home_dir();
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

    // ── MULTI-TURN AGENTIC LOOP (UCU parallel_dispatch aware) ──
    // If the response has tool/exec tags AND agentic loop is enabled,
    // enter multi-turn mode: execute → feed results back to LLM → repeat.
    if llm_result.is_ok() && is_complex && config.runtime.agentic_loop
        && super::agentic_loop_format::has_actionable_tags(&final_response)
    {
        // UCU: compute iteration budget dynamically instead of fixed total
        let iter_budget = crate::cognitive::token_budget::agentic_iteration_budget(
            0, config.runtime.agentic_max_turns, config.runtime.agentic_token_budget, 0,
        );
        eprintln!("[hydra:agentic] UCU budget: {} per iteration (total {})", iter_budget, config.runtime.agentic_token_budget);
        let loop_cfg = agentic_loop_entry::AgenticLoopConfig {
            max_turns: config.runtime.agentic_max_turns.min(10),
            turn_timeout_secs: 30,
            total_budget_tokens: config.runtime.agentic_token_budget,
        };
        // Build a minimal system prompt for subsequent turns
        let sys = "You are Hydra, continuing a multi-turn task. \
            Use <hydra-tool> for sister MCP tools and <hydra-exec> for shell commands. \
            When done, include <hydra-done/> at the end.";
        let loop_result = agentic_loop::run_agentic_loop(
            text, sys, &final_response, &loop_cfg,
            llm_config, active_model, provider, config,
            sisters_handle, decide_engine, undo_stack, db, tx,
        ).await;
        final_response = loop_result.final_response;
        all_exec_results.extend(loop_result.all_exec_results);
        let _ = tx.send(CognitiveUpdate::AgenticComplete {
            turns: loop_result.turns_completed,
            total_tokens: loop_result.total_tokens,
            stop_reason: loop_result.stop_reason.to_string(),
        });
        eprintln!("[hydra:agentic] Loop done: {} turns, reason={}", loop_result.turns_completed, loop_result.stop_reason);
    }

    // ── UCU PARALLEL DISPATCH — track plan step completion + guide agentic loop ──
    if let Some(ref plan) = task_plan {
        let mut graph = crate::cognitive::parallel_dispatch::DependencyGraph::new(&plan.steps);
        let success_count = all_exec_results.iter().filter(|(_, _, s)| *s).count();
        for i in 0..success_count.min(plan.steps.len()) {
            graph.mark_completed(i);
            let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: i, duration_ms: None });
        }
        // Report remaining work to the LLM via response annotation
        if !graph.all_complete() {
            if let Some(batch) = graph.next_batch() {
                let remaining: Vec<String> = batch.step_ids.iter()
                    .filter_map(|&id| plan.steps.get(id).map(|s| s.description.clone()))
                    .collect();
                if !remaining.is_empty() {
                    final_response.push_str(&format!(
                        "\n\n**Next steps ({}/{} complete):**\n{}",
                        graph.completed_count(), graph.total_steps(),
                        remaining.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n"),
                    ));
                }
            }
        }
    }

    // ── RESPONSE VERIFICATION PIPELINE ──
    // Claude-like pattern 8.1: verify before delivery using sisters in parallel.
    if llm_result.is_ok() {
        if let Some(ref sh) = sisters_handle {
            let review_fut = async {
                if is_complex { self_review_response(text, &final_response, llm_config).await } else { None }
            };
            let confidence_fut = sh.veritas_score_confidence(&final_response);
            let consistency_fut = async {
                if let Some(ref mem) = perceive_memory {
                    sh.veritas_check_consistency(&final_response, mem).await
                } else { None }
            };
            let aegis_fut = sh.aegis_validate_output(text, &final_response);
            let (review_r, confidence_r, consistency_r, aegis_r) =
                tokio::join!(review_fut, confidence_fut, consistency_fut, aegis_fut);
            // Self-review
            if let Some(issue) = review_r {
                eprintln!("[hydra:review] Self-review flagged: {}", issue);
                final_response.push_str(&format!("\n\n---\n*Note: {}*", issue));
            }
            // Veritas confidence — add caveat if low
            if let Some(conf) = confidence_r {
                if conf < 0.4 {
                    final_response.push_str("\n\n*I'm not fully confident in this response — please verify.*");
                } else if conf < 0.7 {
                    eprintln!("[hydra:veritas] Moderate confidence: {:.0}%", conf * 100.0);
                }
            }
            // Veritas consistency — flag contradictions
            if let Some(ref contradiction) = consistency_r {
                eprintln!("[hydra:veritas] Consistency issue: {}", contradiction);
                let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                    title: "Consistency Check".into(),
                    content: contradiction.clone(),
                });
            }
            // Aegis output validation
            if let Some(validation) = aegis_r {
                if !validation.safe {
                    eprintln!("[hydra:aegis] Output warning: {}", validation.reason);
                    let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                        title: "Output Security Check".into(),
                        content: format!("[{:?}] {}", validation.severity, validation.reason),
                    });
                }
            }
        } else if is_complex {
            // No sisters — still do self-review for complex queries
            if let Some(issue) = self_review_response(text, &final_response, llm_config).await {
                final_response.push_str(&format!("\n\n---\n*Note: {}*", issue));
            }
        }
    }

    // Sign receipt via Identity sister
    if let Some(ref sh) = sisters_handle {
        if let Some(id) = &sh.identity {
            if let Err(e) = id.call_tool("receipt_create", serde_json::json!({
                "action": text,
                "risk_level": risk_level,
                "gate_decision": gate_decision,
                "tokens_used": input_tokens + output_tokens,
            })).await {
                eprintln!("[hydra:identity] receipt_create FAILED: {}", e);
            }
        }
    }

    // Identity: create execution receipt via delegation
    if let Some(ref sh) = sisters_handle {
        sh.act_receipt(text, risk_level, llm_ok).await;
    }

    // Vision: capture screen state after execution (complex tasks only)
    if is_complex {
        if let Some(ref sh) = sisters_handle {
            if sh.act_vision_capture(text).await.is_none() {
                eprintln!("[hydra:act] act_vision_capture returned None");
            }
        }
    }

    // Record trust outcome — success earns trust, failure loses it
    if llm_result.is_ok() {
        decide_engine.record_success(risk_level, "");
    } else {
        decide_engine.record_failure(risk_level, "");
    }

    // UCU dependency_resolver: detect missing deps from execution failures
    for (cmd, output, success) in &all_exec_results {
        if !*success {
            if let Some(dep) = crate::cognitive::dependency_resolver::detect_missing_dependency(output) {
                let res = crate::cognitive::dependency_resolver::suggest_resolution(&dep);
                eprintln!("[hydra:deps] {:?} '{}' — {:?}", dep.kind, dep.name, res.action_taken);
                if let crate::cognitive::dependency_resolver::ResolutionAction::SuggestInstall(ref install_cmd) = res.action_taken {
                    final_response.push_str(&format!("\n\n**Missing dependency:** {}\nSuggested fix: `{}`", res.description, install_cmd));
                }
            }
        }
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

    // Phase 5, P2: Obstacle resolution + UCU backtrack analysis
    {
        let mut resolver = super::obstacle_handler::create_resolver();
        for (idx, (cmd, output, success)) in all_exec_results.iter().enumerate() {
            if !*success {
                // UCU backtrack: analyze failure for root cause fix suggestions
                if let Some(fix) = crate::cognitive::backtrack::suggest_fix(output) {
                    eprintln!("[hydra:backtrack] step={} fix_hint={}", idx, fix);
                }
                if let Some(resolution) = super::obstacle_handler::try_resolve_obstacle(
                    output, cmd, &mut resolver, llm_config, tx,
                ).await {
                    let obstacle = crate::cognitive::obstacles::Obstacle::from_error(output, cmd);
                    let msg = super::obstacle_handler::format_resolution_message(
                        obstacle.pattern.label(), &resolution.summary(), output,
                    );
                    final_response.push_str(&format!("\n\n{}", msg));
                }
            }
        }
    }

    // Planning: update goal progress
    if let Some(ref goal_id) = planning_goal_id {
        if let Some(ref sh) = sisters_handle {
            let status = if all_exec_results.iter().all(|(_, _, s)| *s) { "progressing" } else { "partial" };
            let step_idx = all_exec_results.len().min(10);
            sh.planning_update_progress(goal_id, step_idx, status).await;
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
