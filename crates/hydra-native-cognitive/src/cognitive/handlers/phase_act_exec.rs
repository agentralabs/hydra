//! ACT phase — command execution, security checks, retry, receipt persistence.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::cognitive::decide::DecideEngine;
use crate::sisters::SistersHandle;
use hydra_native_state::utils::safe_truncate;
use hydra_db::HydraDb;
use hydra_runtime::undo::UndoStack;

use super::super::loop_runner::CognitiveUpdate;
use super::actions::{extract_inline_commands, detect_direct_action_command, extract_url_from_command, strip_hydra_exec_tags};
use crate::sisters::tool_dispatch::{extract_hydra_tool_tags, strip_hydra_tool_tags};
use super::llm_helpers::{commands_are_dependent, diagnose_and_retry};
use super::memory::md5_simple;
use super::platform_system::detect_system_control;

/// Execute all inline commands through the full security pipeline.
///
/// Returns `(updated_final_response, all_exec_results)`.
pub(crate) async fn execute_commands(
    text: &str,
    final_response: &str,
    config: &super::super::loop_runner::CognitiveLoopConfig,
    llm_config: &hydra_model::LlmConfig,
    decide_engine: &Arc<DecideEngine>,
    sisters_handle: &Option<SistersHandle>,
    _undo_stack: &Option<Arc<parking_lot::Mutex<UndoStack>>>,
    db: &Option<Arc<HydraDb>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> (String, Vec<(String, String, bool)>) {
    let mut updated_response = final_response.to_string();
    let mut exec_results = Vec::new();

    // Strategy 1: Parse <hydra-exec> tags
    let tagged_commands = extract_inline_commands(final_response);

    // Strategy 2: Direct intent detection
    let direct_cmd = if tagged_commands.is_empty() {
        detect_direct_action_command(text).or_else(|| detect_system_control(text))
    } else { None };

    let all_commands: Vec<String> = tagged_commands.into_iter()
        .chain(direct_cmd.into_iter())
        .collect();

    // Phase 2, D3: Compound Risk Scoring for multi-command batches
    if all_commands.len() > 1 {
        let (compound_score, compound_level, compound_detail) =
            decide_engine.compound_risk_score(&all_commands);
        eprintln!("[hydra:compound] {} commands, compound risk: {:.2} ({})",
            all_commands.len(), compound_score, compound_level);
        if compound_score >= 0.7 {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Compound Risk Warning".to_string(),
                content: format!(
                    "Multi-command batch risk: {:.0}% ({}). Details: {}",
                    compound_score * 100.0, compound_level, compound_detail
                ),
            });
        }
    }

    // Phase 2, A3: Multi-Step Execution Chaining
    // Track whether previous commands failed so we can skip dependent commands.
    let mut previous_failed = false;
    let mut failed_outputs: Vec<String> = Vec::new();

    for cmd in &all_commands {
        // Phase 2, A3: Skip dependent commands if a previous command failed.
        if previous_failed && commands_are_dependent(
            failed_outputs.last().map(|s| s.as_str()).unwrap_or(""),
            cmd,
        ) {
            eprintln!("[hydra:chain] Skipping '{}' — depends on failed previous command", safe_truncate(cmd, 60));
            exec_results.push((cmd.clone(), "SKIPPED — previous dependent command failed".to_string(), false));
            continue;
        }
        // ══════════════════════════════════════════════════════════
        // FULL SECURITY PIPELINE: Local → Anomaly → Boundary → Risk → Gate
        // ══════════════════════════════════════════════════════════

        // Layer 0: Local deny-list warning — warn but allow execution
        if crate::sisters::aegis_deep::is_locally_blocked(cmd) {
            eprintln!("[hydra:SECURITY] ⚠ DANGEROUS COMMAND WARNING: {}", safe_truncate(cmd, 60));
            let _ = tx.send(CognitiveUpdate::ShadowValidation {
                safe: false,
                recommendation: format!("⚠ Dangerous command detected: {}. Proceeding with caution.", safe_truncate(cmd, 60)),
            });
        }

        // Layer 1-3: evaluate_command does anomaly detection, boundary
        // enforcement, and risk assessment in one call
        let gate_result = decide_engine.evaluate_command(cmd);

        // Also check trust-based autonomy
        let _cmd_decide = decide_engine.check(&gate_result.risk_level, cmd);

        // Create receipt BEFORE execution (audit trail)
        if let Some(ref sh) = sisters_handle {
            sh.act_receipt(cmd, &gate_result.risk_level, gate_result.allowed).await;
        }

        // ── WARN: Anomaly detected (burst, exfiltration, destructive) ──
        if gate_result.anomaly_detected {
            let is_critical = gate_result.reason.contains("CRITICAL") || gate_result.reason.contains("exfiltration");
            if let Some(ref db) = db {
                let _ = db.create_anomaly_event(&hydra_db::AnomalyEventRow {
                    event_type: if is_critical { "critical".into() } else { "anomaly".into() },
                    command: cmd.clone(),
                    detail: Some(gate_result.reason.clone()),
                    severity: if is_critical { "critical".into() } else { "high".into() },
                    kill_switch_engaged: false,
                });
            }
            let _ = tx.send(CognitiveUpdate::ShadowValidation {
                safe: false, recommendation: gate_result.reason.clone(),
            });
            eprintln!("[hydra:SECURITY] ⚠ ANOMALY WARNING: {} — proceeding", safe_truncate(&gate_result.reason, 80));
        }

        // ── WARN: Boundary violation (system paths, self-modification) ──
        if gate_result.boundary_blocked {
            eprintln!("[hydra:SECURITY] ⚠ BOUNDARY WARNING: {} — proceeding", safe_truncate(&gate_result.reason, 80));
            let _ = tx.send(CognitiveUpdate::ShadowValidation {
                safe: false,
                recommendation: format!("⚠ Boundary: {}", safe_truncate(&gate_result.reason, 60)),
            });
        }

        // ── RISK SCORE LOGGING (no blocking) ──
        if gate_result.risk_score >= 0.5 {
            eprintln!("[hydra:security] Elevated risk {:.2} for: {}", gate_result.risk_score, safe_truncate(cmd, 80));
        }

        // ── Aegis shadow validation for elevated risk (0.3+) ──
        if gate_result.risk_score >= 0.3 {
            if let Some(ref sh) = sisters_handle {
                if let Some((safe, rec)) = sh.act_aegis_validate(cmd).await {
                    // Persist shadow validation to DB
                    if let Some(ref db) = db {
                        if let Err(e) = db.create_shadow_validation(&hydra_db::ShadowValidationRow {
                            action_description: cmd.clone(),
                            safe,
                            divergence_count: if safe { 0 } else { 1 },
                            critical_divergences: if safe { 0 } else { 1 },
                            recommendation: Some(rec.clone()),
                        }) {
                            eprintln!("[hydra:SECURITY] Failed to persist shadow validation: {}", e);
                        }
                    }
                    if !safe {
                        eprintln!("[hydra:SECURITY] ⚠ AEGIS WARNING: {} — proceeding", safe_truncate(&rec, 60));
                        let _ = tx.send(CognitiveUpdate::ShadowValidation {
                            safe: false, recommendation: rec.clone(),
                        });
                    }
                }
            }
        }

        // ═══ ALL GATES PASSED — EXECUTE ═══
        // Auto-background long-running services (dev servers, watchers, etc.)
        if super::exec_engine::should_background(cmd) {
            let bg_id = super::exec_engine::spawn_background(cmd, tx);
            exec_results.push((cmd.clone(), format!("Running in background [{}]", bg_id), true));
            continue;
        }
        let _ = tx.send(CognitiveUpdate::Phase(format!("Executing: {}", cmd)));

        // Ghost cursor: Show for visual actions (open, browse, UI interaction)
        let is_visual_cmd = cmd.contains("open -a") || cmd.contains("open http")
            || cmd.contains("xdg-open") || cmd.starts_with("open ")
            || cmd.contains("google-chrome") || cmd.contains("firefox");
        let cursor_session_id = if is_visual_cmd { Some(uuid::Uuid::new_v4().to_string()) } else { None };
        let cursor_start = std::time::Instant::now();
        if is_visual_cmd {
            if let (Some(ref sess_id), Some(ref db)) = (&cursor_session_id, &db) {
                let _ = db.create_cursor_session(sess_id, &config.task_id, "execute");
            }
            let _ = tx.send(CognitiveUpdate::CursorVisibility { visible: true });
            // Animate cursor to center-ish of screen with action label
            let label = if cmd.contains("open -a") || cmd.contains("open ") {
                let app = cmd.split("open -a ").nth(1)
                    .or_else(|| cmd.split("open ").nth(1))
                    .unwrap_or(cmd)
                    .trim_matches('"');
                format!("Opening {}", app)
            } else {
                "Navigating...".into()
            };
            let _ = tx.send(CognitiveUpdate::CursorMove { x: 400.0, y: 300.0, label: Some(label) });
            let _ = tx.send(CognitiveUpdate::CursorClick);
        }

        // 5-minute timeout — long enough for builds, deploys, large downloads
        let cmd_future = super::exec_engine::run_command(cmd);
        match tokio::time::timeout(Duration::from_secs(300), cmd_future).await {
            Err(_elapsed) => {
                eprintln!("[hydra:exec] Command timed out after 5m: {}", safe_truncate(cmd, 60));
                decide_engine.record_failure(&gate_result.risk_level, cmd);
                exec_results.push((cmd.clone(), "TIMEOUT — command exceeded 5 minute limit".into(), false));
                if is_visual_cmd {
                    let _ = tx.send(CognitiveUpdate::CursorVisibility { visible: false });
                    if let (Some(ref sess_id), Some(ref db)) = (&cursor_session_id, &db) {
                        let dur = cursor_start.elapsed().as_millis() as i64;
                        let _ = db.finish_cursor_session(sess_id, 0, dur);
                    }
                }
                previous_failed = true;
                failed_outputs.push("TIMEOUT".to_string());
                continue;
            }
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                // 256KB output buffer — big enough for real build logs
                let raw_combined = if stderr.is_empty() { stdout } else if stdout.is_empty() { stderr } else { format!("{}\n{}", stdout, stderr) };
                let combined = safe_truncate(&raw_combined, 262_144).to_string();
                // Warn about secrets in output but don't redact — Hydra needs to see everything
                if crate::sisters::aegis_deep::output_contains_secrets(&combined) {
                    eprintln!("[hydra:SECURITY] ⚠ Secrets detected in output of: {}", safe_truncate(cmd, 60));
                    let _ = tx.send(CognitiveUpdate::ShadowValidation {
                        safe: false,
                        recommendation: "⚠ Command output may contain secrets — handle with care".into(),
                    });
                }
                let success = output.status.success();
                exec_results.push((cmd.clone(), combined.clone(), success));

                // Record trust outcome
                if success {
                    decide_engine.record_success(&gate_result.risk_level, cmd);
                } else {
                    // Phase 2, A2: Retry-with-Diagnosis — attempt ONE retry on failure
                    eprintln!("[hydra:retry] Command failed, attempting diagnosis...");
                    if let Some((fix_cmd, fix_output, fix_success)) =
                        diagnose_and_retry(cmd, &combined, llm_config, decide_engine).await
                    {
                        eprintln!("[hydra:retry] Retry result: {} (success={})", safe_truncate(&fix_cmd, 60), fix_success);
                        // Replace the failed result with the retry result
                        exec_results.pop(); // Remove the failed entry we just pushed
                        exec_results.push((
                            format!("{} → retried: {}", cmd, fix_cmd),
                            fix_output,
                            fix_success,
                        ));
                        if fix_success {
                            decide_engine.record_success(&gate_result.risk_level, &fix_cmd);
                        }
                        // Skip recording failure for original if retry succeeded
                        if !fix_success {
                            decide_engine.record_failure(&gate_result.risk_level, cmd);
                        }
                    } else {
                        decide_engine.record_failure(&gate_result.risk_level, cmd);
                    }
                }

                // Persist receipt to DB (hash-chained audit trail)
                if let Some(ref db) = db {
                    let seq = db.next_receipt_sequence().unwrap_or(1);
                    let prev = db.last_receipt_hash().unwrap_or(None);
                    let hash_input = format!("{}:{}:{}:{}", seq, cmd, success, prev.as_deref().unwrap_or("genesis"));
                    let hash = format!("{:x}", md5_simple(&hash_input));
                    if let Err(e) = db.create_receipt(&hydra_db::ReceiptRow {
                        id: uuid::Uuid::new_v4().to_string(),
                        receipt_type: if success { "execution_success".into() } else { "execution_failure".into() },
                        action: cmd.clone(),
                        actor: "hydra".into(),
                        tokens_used: 0,
                        risk_level: Some(gate_result.risk_level.clone()),
                        hash,
                        prev_hash: prev,
                        sequence: seq,
                        created_at: chrono::Utc::now().to_rfc3339(),
                    }) {
                        eprintln!("[hydra:SECURITY] Failed to persist receipt to DB: {} — cmd={}", e, safe_truncate(cmd, 60));
                    }
                }

                // ── LEARN: Capture every command execution in memory ──
                if let Some(ref sh) = sisters_handle {
                    sh.learn_capture_command(cmd, &combined, success).await;
                }

                // Ghost cursor: Hide after visual command completes
                if is_visual_cmd {
                    let _ = tx.send(CognitiveUpdate::CursorVisibility { visible: false });
                }

                // Record cursor event and finish cursor session
                if is_visual_cmd {
                    if let Some(ref db) = db {
                        let _ = db.record_cursor_event(
                            &config.task_id, 0, "execute",
                            400.0, 300.0,
                            Some(&serde_json::json!({
                                "command": cmd,
                                "success": success,
                            }).to_string()),
                        );
                    }
                    if let (Some(ref sess_id), Some(ref db)) = (&cursor_session_id, &db) {
                        let dur = cursor_start.elapsed().as_millis() as i64;
                        let _ = db.finish_cursor_session(sess_id, 1, dur);
                    }
                }
            }
            Ok(Err(e)) => {
                decide_engine.record_failure(&gate_result.risk_level, cmd);
                exec_results.push((cmd.clone(), format!("Failed: {}", e), false));
                if is_visual_cmd {
                    let _ = tx.send(CognitiveUpdate::CursorVisibility { visible: false });
                    if let (Some(ref sess_id), Some(ref db)) = (&cursor_session_id, &db) {
                        let dur = cursor_start.elapsed().as_millis() as i64;
                        let _ = db.finish_cursor_session(sess_id, 0, dur);
                    }
                }
            }
        }

        // Phase 2, A3: Track failure state for dependency chaining
        if let Some((_, ref output, success)) = exec_results.last() {
            if !success {
                previous_failed = true;
                failed_outputs.push(output.clone());
            } else {
                previous_failed = false;
            }
        }
    }

    if !exec_results.is_empty() {
        let cleaned = strip_hydra_exec_tags(&updated_response);
        updated_response = cleaned;
        for (cmd, output, success) in &exec_results {
            if !output.trim().is_empty() {
                updated_response.push_str(&format!(
                    "\n\n```\n$ {}\n{}\n```",
                    cmd,
                    output.trim()
                ));
            }
            if !success {
                updated_response.push_str(&format!("\n*(Command `{}` failed)*", cmd));
            }
        }
    }

    // ── Vision: full browser agent pipeline after URL navigation ──
    if let Some(ref sh) = sisters_handle {
        for (cmd, _, success) in &exec_results {
            if *success && (cmd.contains("http://") || cmd.contains("https://") || cmd.contains("open -a")) {
                if let Some(url) = extract_url_from_command(cmd) {
                    // L0: DOM extraction first (zero vision tokens)
                    if let Some(obs) = sh.browse_navigate(&url).await {
                        let mut web_parts = Vec::new();
                        if let Some(ref t) = obs.title { web_parts.push(format!("**{}**", t)); }
                        if let Some(ref c) = obs.content { web_parts.push(safe_truncate(c, 400).to_string()); }
                        if let Some(ref i) = obs.interactive_elements { web_parts.push(format!("Interactive: {}", safe_truncate(i, 200))); }
                        if let Some(ref f) = obs.forms { web_parts.push(format!("Forms: {}", safe_truncate(f, 200))); }
                        if !web_parts.is_empty() {
                            updated_response.push_str(&format!("\n\n**Web page (DOM):**\n{}\n", web_parts.join("\n")));
                        }
                        // Learn grammar for future zero-token visits
                        if sh.browse_learn_grammar(&url).await.is_none() {
                            eprintln!("[hydra:act] browse_learn_grammar({}) returned None", url);
                        }
                    } else if let Some(web_content) = sh.act_vision_capture(&url).await {
                        // Fallback: old-style web map if DOM extraction unavailable
                        updated_response.push_str(&format!(
                            "\n\n**Web page captured:**\n{}\n",
                            safe_truncate(&web_content, 500)
                        ));
                    }
                }
            }
        }
    }

    // ── MCP Tool Dispatch: execute <hydra-tool> tags from LLM output ──
    if let Some(ref sh) = sisters_handle {
        let tool_results = sh.execute_tool_tags(&updated_response).await;
        if !tool_results.is_empty() {
            updated_response = strip_hydra_tool_tags(&updated_response);
            for (name, output) in &tool_results {
                updated_response.push_str(&format!(
                    "\n\n**{}:**\n{}\n", name, safe_truncate(output, 1000)
                ));
            }
        }
    }

    (updated_response, exec_results)
}
