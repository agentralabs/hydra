//! Pre-phase action dispatch handlers — crystallized skills, dep queries, slash commands, direct actions.
//!
//! Each handler returns `true` if it handled the intent (caller should return early),
//! or `false` to fall through to the next handler / 5-phase loop.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::cognitive::decide::DecideEngine;
use crate::cognitive::inventions::InventionEngine;
use crate::sisters::SistersHandle;
use hydra_native_state::utils::safe_truncate;

use crate::cognitive::loop_runner::{CognitiveLoopConfig, CognitiveUpdate};
use super::actions::{detect_direct_action_command, format_command_output};
use super::llm_helpers::handle_universal_slash_command;
use super::platform_system::detect_system_control;

/// Run a shell command and return (formatted_output, raw_output).
async fn run_shell_cmd(cmd: &str) -> Result<(String, String), String> {
    match tokio::process::Command::new("sh").arg("-c").arg(cmd).output().await {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let raw = if stderr.is_empty() { stdout.to_string() }
                else if stdout.is_empty() { stderr.to_string() }
                else { format!("{}\n{}", stdout, stderr) };
            Ok((format_command_output(&raw), raw))
        }
        Err(e) => Err(format!("Command failed: {}", e)),
    }
}

/// Handle crystallized skill shortcut — bypass LLM for learned patterns.
/// Returns `true` if the skill was handled (slash-command variant).
pub(crate) async fn handle_crystallized_skill(
    text: &str,
    inventions: &Option<Arc<InventionEngine>>,
    decide_engine: &Arc<DecideEngine>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let inv = match inventions.as_ref() {
        Some(inv) => inv,
        None => return false,
    };

    let (skill_name, skill_actions) = match inv.match_crystallized_skill(text) {
        Some(pair) => pair,
        None => return false,
    };

    eprintln!("[hydra:crystal] Matched crystallized skill '{}' — bypassing LLM", skill_name);
    let _ = tx.send(CognitiveUpdate::Phase("Act (crystallized)".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    // Extract the shell command from the action chain (act:respond or act:execute_plan)
    let cmd = skill_actions.iter()
        .find(|a| a.starts_with("act:"))
        .map(|a| a.strip_prefix("act:").unwrap_or(a).to_string());

    // For slash-command skills, re-execute the original command
    if text.starts_with('/') {
        if let Some(slash_result) = handle_universal_slash_command(text) {
            if !slash_result.starts_with("__TEXT__:") {
                // Gate check before executing crystallized skill command
                let gate_result = decide_engine.evaluate_command(&slash_result);
                if gate_result.risk_score >= 0.5 || gate_result.anomaly_detected || gate_result.boundary_blocked {
                    eprintln!("[hydra:dispatch] ⚠ Skill `{}` risk warning: {} — proceeding", skill_name, gate_result.reason);
                }
                match run_shell_cmd(&slash_result).await {
                    Ok((_, raw)) => {
                        let _ = tx.send(CognitiveUpdate::Message {
                            role: "hydra".into(),
                            content: format!("⚡ *Crystallized skill `{}`*\n\n```\n{}\n```", skill_name, raw.trim()),
                            css_class: "message hydra".into(),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(CognitiveUpdate::Message {
                            role: "hydra".into(), content: format!("Crystallized skill `{}` failed: {}", skill_name, e),
                            css_class: "message hydra error".into(),
                        });
                    }
                }
            } else {
                let content = slash_result.strip_prefix("__TEXT__:").unwrap_or(&slash_result);
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: content.to_string(),
                    css_class: "message hydra".into(),
                });
            }
            let _ = tx.send(CognitiveUpdate::SkillCrystallized {
                name: skill_name,
                actions_count: skill_actions.len(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            return true;
        }
    }

    // For non-slash crystallized skills, note it and fall through to normal processing
    eprintln!("[hydra:crystal] Non-slash skill '{}' — proceeding with LLM (cmd={:?})", skill_name, cmd);
    false
}

/// Handle dependency/usage queries pre-check — detect before misrouting to MemoryRecall.
pub(crate) async fn handle_dep_query_precheck(
    text: &str,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let lower_precheck = text.to_lowercase();
    let is_dep_query = lower_precheck.contains("depends on") || lower_precheck.contains("what depends")
        || lower_precheck.contains("who uses") || lower_precheck.contains("what imports")
        || lower_precheck.contains("impact of") || lower_precheck.contains("what calls")
        || lower_precheck.contains("who calls") || lower_precheck.contains("references to")
        || lower_precheck.contains("what uses") || lower_precheck.contains("who imports")
        || (lower_precheck.contains("where is") && lower_precheck.contains("used"))
        || (lower_precheck.contains("search") && (lower_precheck.contains("codebase")
            || lower_precheck.contains("in the code") || lower_precheck.contains("in my code")));
    if !is_dep_query {
        return false;
    }

    let direct_cmd = match detect_direct_action_command(text) {
        Some(cmd) => cmd,
        None => return false,
    };

    let phase = if direct_cmd.contains("grep -rn") {
        "Searching codebase..."
    } else {
        "Executing..."
    };
    let _ = tx.send(CognitiveUpdate::Phase(phase.into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));
    let _ = tx.send(CognitiveUpdate::Typing(false));
    eprintln!("[hydra:direct-precheck] Executing: {}", safe_truncate(&direct_cmd, 100));
    match run_shell_cmd(&direct_cmd).await {
        Ok((display, raw)) => {
            let _ = tx.send(CognitiveUpdate::Message { role: "hydra".into(), content: display, css_class: "message hydra".into() });
            if let Some(ref sh) = sisters_handle { sh.learn(text, safe_truncate(&raw, 500)).await; }
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::Message { role: "hydra".into(), content: e, css_class: "message hydra error".into() });
        }
    }
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

/// Handle slash commands — /test, /files, /git, /build, /run, etc.
pub(crate) async fn handle_slash_command(
    text: &str,
    decide_engine: &Arc<DecideEngine>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if !text.starts_with('/') {
        return false;
    }
    let slash_result = match handle_universal_slash_command(text) {
        Some(r) => r,
        None => return false,
    };

    let _ = tx.send(CognitiveUpdate::Phase("Act (command)".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    // Project execution marker — trigger ProjectExecutor pipeline
    if slash_result.starts_with("__TEXT__:__PROJECT_EXEC__:") {
        let marker = &slash_result["__TEXT__:__PROJECT_EXEC__:".len()..];
        let (mode, url) = marker.split_once(':').unwrap_or(("EXECUTE", marker));
        run_project_executor(url, mode == "DRY RUN", None, tx).await;
        return true;
    }

    // Memory mode change — emits MemoryModeChanged for UI to handle
    if slash_result.starts_with("__TEXT__:__MEMORY_MODE__:") {
        let rest = &slash_result["__TEXT__:__MEMORY_MODE__:".len()..];
        let (mode, msg) = rest.split_once(':').unwrap_or(("all", "Memory mode changed"));
        let _ = tx.send(CognitiveUpdate::MemoryModeChanged { mode: mode.into() });
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: msg.to_string(),
            css_class: "message hydra settings-applied".into(),
        });
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return true;
    }

    // Some slash commands return static text (no shell execution needed)
    if slash_result.starts_with("__TEXT__:") {
        let content = slash_result.strip_prefix("__TEXT__:").unwrap_or(&slash_result);
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: content.to_string(),
            css_class: "message hydra".into(),
        });
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return true;
    }

    // Gate check — warn only
    let gate_result = decide_engine.evaluate_command(&slash_result);
    if gate_result.risk_score >= 0.5 || gate_result.anomaly_detected || gate_result.boundary_blocked {
        eprintln!("[hydra:dispatch] ⚠ Slash command risk warning: {} — proceeding", gate_result.reason);
    }

    // Execute the shell command
    eprintln!("[hydra:slash] Executing: {}", safe_truncate(&slash_result, 100));
    match run_shell_cmd(&slash_result).await {
        Ok((display, _)) => {
            let _ = tx.send(CognitiveUpdate::Message { role: "hydra".into(), content: display, css_class: "message hydra".into() });
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::Message { role: "hydra".into(), content: e, css_class: "message hydra error".into() });
        }
    }
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

/// Handle direct action fast-path — execute immediately, skip LLM.
pub(crate) async fn handle_direct_action(
    text: &str,
    sisters_handle: &Option<SistersHandle>,
    decide_engine: &Arc<DecideEngine>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let direct_result = detect_direct_action_command(text)
        .or_else(|| detect_system_control(text));
    eprintln!("[hydra:loop] direct_action_check: {:?}", direct_result.as_ref().map(|c| safe_truncate(c, 80)));
    let direct_cmd = match direct_result {
        Some(cmd) => cmd,
        None => return false,
    };

    // Send specific phase based on what we're doing
    let phase = if direct_cmd.contains("grep -rn") {
        "Searching codebase..."
    } else if direct_cmd.contains("curl") {
        "Fetching from web..."
    } else if direct_cmd.contains("osascript") || direct_cmd.contains("xdg-open") {
        "Opening app..."
    } else {
        "Executing..."
    };
    let _ = tx.send(CognitiveUpdate::Phase(phase.into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    // Risk check — warn only
    let gate_result = decide_engine.evaluate_command(&direct_cmd);
    if gate_result.risk_score >= 0.5 || gate_result.anomaly_detected || gate_result.boundary_blocked {
        eprintln!("[hydra:dispatch] ⚠ Direct command risk warning: {} — proceeding", gate_result.reason);
    }

    // Execute the command directly
    let _ = tx.send(CognitiveUpdate::Typing(false));
    eprintln!("[hydra:direct] Executing: {}", safe_truncate(&direct_cmd, 100));
    match run_shell_cmd(&direct_cmd).await {
        Ok((display, raw)) => {
            let _ = tx.send(CognitiveUpdate::Message { role: "hydra".into(), content: display, css_class: "message hydra".into() });
            if let Some(ref sh) = sisters_handle { sh.learn(text, safe_truncate(&raw, 500)).await; }
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::Message { role: "hydra".into(), content: e, css_class: "message hydra error".into() });
        }
    }

    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

/// Handle natural language project execution requests (e.g., "test https://github.com/user/repo").
pub(crate) async fn handle_project_exec_natural(
    text: &str,
    sisters_handle: &Option<crate::sisters::SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if !crate::project_exec::is_project_exec_request(text) {
        return false;
    }
    let url = match crate::project_exec::extract_url(text) {
        Some(u) => u,
        None => return false,
    };
    let dry_run = {
        let lower = text.to_lowercase();
        lower.contains("dry") && lower.contains("run")
    };
    run_project_executor(&url, dry_run, sisters_handle.as_ref(), tx).await;
    true
}

/// Spawn ProjectExecutor and forward progress updates to the cognitive loop.
async fn run_project_executor(
    url: &str,
    dry_run: bool,
    sisters_handle: Option<&crate::sisters::SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let request = if dry_run {
        crate::project_exec::ProjectRequest::dry_run(url)
    } else {
        crate::project_exec::ProjectRequest::new(url)
    };

    let _ = tx.send(CognitiveUpdate::Phase("Project Execution".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    let (exec_tx, mut exec_rx) = mpsc::channel::<CognitiveUpdate>(100);
    let tx_fwd = tx.clone();

    // Forward progress updates from executor channel to cognitive loop
    let forwarder = tokio::spawn(async move {
        while let Some(update) = exec_rx.recv().await {
            let _ = tx_fwd.send(update);
        }
    });

    // Run executor in blocking task (it does git clone, runs shell commands)
    let report = tokio::task::spawn_blocking(move || {
        let mut executor = crate::project_exec::ProjectExecutor::new();
        executor.execute(&request, &exec_tx)
    }).await;

    let _ = forwarder.await;

    match report {
        Ok(r) => {
            let table = r.detailed_table();
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: table.clone(),
                css_class: "message hydra".into(),
            });
            // Store test result as episode in Memory for later recall
            if let Some(sh) = sisters_handle {
                let summary = r.one_line_summary();
                sh.memory_store_episode(&summary, "project_test").await;
                sh.memory_capture_exchange(
                    &format!("test {}", url),
                    &summary,
                ).await;
            }
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: format!("Project execution failed: {}", e),
                css_class: "message hydra error".into(),
            });
        }
    }
    let _ = tx.send(CognitiveUpdate::ResetIdle);
}
