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

/// Handle crystallized skill shortcut — bypass LLM for learned patterns.
/// Returns `true` if the skill was handled (slash-command variant).
pub(crate) async fn handle_crystallized_skill(
    text: &str,
    inventions: &Option<Arc<InventionEngine>>,
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
                match tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&slash_result)
                    .output()
                    .await
                {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let combined = if stderr.is_empty() { stdout.to_string() }
                            else if stdout.is_empty() { stderr.to_string() }
                            else { format!("{}\n{}", stdout, stderr) };
                        let _ = tx.send(CognitiveUpdate::Message {
                            role: "hydra".into(),
                            content: format!("⚡ *Crystallized skill `{}`*\n\n```\n{}\n```",
                                skill_name, combined.trim()),
                            css_class: "message hydra".into(),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(CognitiveUpdate::Message {
                            role: "hydra".into(),
                            content: format!("Crystallized skill `{}` failed: {}", skill_name, e),
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
    match tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&direct_cmd)
        .output()
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = if stderr.is_empty() {
                stdout.to_string()
            } else if stdout.is_empty() {
                stderr.to_string()
            } else {
                format!("{}\n{}", stdout, stderr)
            };
            let display = format_command_output(&combined);
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: display,
                css_class: "message hydra".into(),
            });
            if let Some(ref sh) = sisters_handle {
                sh.learn(text, safe_truncate(&combined, 500)).await;
            }
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: format!("Command failed: {}", e),
                css_class: "message hydra error".into(),
            });
        }
    }
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

/// Handle slash commands — /test, /files, /git, /build, /run, etc.
pub(crate) async fn handle_slash_command(
    text: &str,
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

    // Execute the shell command
    eprintln!("[hydra:slash] Executing: {}", safe_truncate(&slash_result, 100));
    match tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&slash_result)
        .output()
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = if stderr.is_empty() {
                stdout.to_string()
            } else if stdout.is_empty() {
                stderr.to_string()
            } else {
                format!("{}\n{}", stdout, stderr)
            };
            let display = format_command_output(&combined);
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: display,
                css_class: "message hydra".into(),
            });
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: format!("Command failed: {}", e),
                css_class: "message hydra error".into(),
            });
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

    // Quick risk check — only block truly dangerous commands
    let gate_result = decide_engine.evaluate_command(&direct_cmd);
    if gate_result.risk_score >= 0.9 || gate_result.anomaly_detected || gate_result.boundary_blocked {
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: format!("Blocked: {}", gate_result.reason),
            css_class: "message hydra error".into(),
        });
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return true;
    }

    // Execute the command directly
    let _ = tx.send(CognitiveUpdate::Typing(false));
    eprintln!("[hydra:direct] Executing: {}", safe_truncate(&direct_cmd, 100));
    match tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&direct_cmd)
        .output()
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = if stderr.is_empty() {
                stdout.to_string()
            } else if stdout.is_empty() {
                stderr.to_string()
            } else {
                format!("{}\n{}", stdout, stderr)
            };
            let display = format_command_output(&combined);
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: display,
                css_class: "message hydra".into(),
            });

            // LEARN: capture this in memory
            if let Some(ref sh) = sisters_handle {
                sh.learn(text, safe_truncate(&combined, 500)).await;
            }
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: format!("Command failed: {}", e),
                css_class: "message hydra error".into(),
            });
        }
    }

    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}
