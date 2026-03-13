//! Self-implement handler — self-modification pipeline with agentic retry loop.
//!
//! Flow: read spec -> approve -> gap analysis (LLM) -> patch gen (LLM) -> apply -> retry on failure -> report.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::sisters::SistersHandle;
use hydra_runtime::approval::{ApprovalDecision, ApprovalManager};

use super::super::super::loop_runner::{CognitiveLoopConfig, CognitiveUpdate};
use super::super::super::intent_router::{IntentCategory, ClassifiedIntent};

/// Handle self-implement — self-modification pipeline (Phase 5, Priority 1).
///
/// Flow: read spec -> approve -> Forge/LLM gap analysis -> Forge/LLM patch gen -> apply with retry -> report.
pub(crate) async fn handle_self_implement(
    text: &str,
    intent: &ClassifiedIntent,
    config: &CognitiveLoopConfig,
    sisters_handle: &Option<SistersHandle>,
    approval_manager: &Option<Arc<ApprovalManager>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    // Match on intent classification OR keyword pattern (LLM may misclassify)
    let lower = text.to_lowercase();
    let keyword_match = lower.contains("implement spec")
        || (lower.contains("implement") && (lower.contains(".md") || lower.contains(".txt")));

    if intent.category != IntentCategory::SelfImplement && !keyword_match {
        return false;
    }

    let has_spec_file = hydra_kernel::self_modify_llm::extract_spec_path(text).is_some();
    if !has_spec_file && !keyword_match {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("SelfImplement".into()));
    let _ = tx.send(CognitiveUpdate::IconState("needs-attention".into()));

    // -- Step 1: Read spec --
    let repo_root = std::env::current_dir().unwrap_or_default();
    let spec_content = match read_spec(text, &repo_root, tx) {
        Some(s) => s,
        None => return true,
    };

    // -- Step 2: Request human approval --
    let approved = request_approval(text, config, approval_manager, tx).await;
    if !approved {
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return true;
    }

    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    // -- Step 3: Build LLM config --
    let llm_config = hydra_model::LlmConfig::from_env_with_overlay(
        &config.anthropic_key,
        &config.openai_key,
        config.anthropic_oauth_token.as_deref(),
    );

    // -- Step 4: Gap analysis --
    let (gaps, gap_summary) = match run_gap_analysis(
        sisters_handle, &spec_content, &llm_config, &repo_root, tx,
    ).await {
        Some(g) => g,
        None => return true,
    };

    // -- Step 5: Initial patch generation --
    let patches = match generate_initial_patches(
        sisters_handle, &spec_content, &gaps, &llm_config, &repo_root, tx,
    ).await {
        Some(p) => p,
        None => return true,
    };

    // -- Step 6-7: Apply patches with agentic retry loop --
    apply_with_retry(patches, gaps, &spec_content, &llm_config, &repo_root, tx).await;

    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

/// Read spec from file path or inline text. Returns None on error (already reported).
fn read_spec(
    text: &str,
    repo_root: &std::path::Path,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Option<String> {
    if let Some(spec_path) = hydra_kernel::self_modify_llm::extract_spec_path(text) {
        let full_path = repo_root.join(&spec_path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: format!("Read spec: **{}** ({} bytes)", spec_path.display(), content.len()),
                    css_class: "message hydra thinking".into(),
                });
                Some(content)
            }
            Err(e) => {
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: format!("Cannot read spec file `{}`: {}", full_path.display(), e),
                    css_class: "message hydra error".into(),
                });
                let _ = tx.send(CognitiveUpdate::ResetIdle);
                None
            }
        }
    } else {
        Some(text.to_string())
    }
}

/// Run gap analysis (Forge first, LLM fallback). Returns None if no gaps found (already reported).
async fn run_gap_analysis(
    sisters_handle: &Option<SistersHandle>,
    spec_content: &str,
    llm_config: &hydra_model::LlmConfig,
    repo_root: &std::path::Path,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Option<(Vec<hydra_kernel::self_modify::SpecGap>, String)> {
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: "Analyzing spec for implementation gaps...".into(),
        css_class: "message hydra thinking".into(),
    });

    let forge_gap_result = try_forge_analyze(sisters_handle, spec_content).await;
    let gaps = match hydra_kernel::self_modify_llm::analyze_spec_gaps(
        spec_content, forge_gap_result, llm_config, repo_root,
    ).await {
        Ok(g) => g,
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: format!("Gap analysis failed: {}. Falling back to pattern matching...", e),
                css_class: "message hydra error".into(),
            });
            let pipeline = hydra_kernel::self_modify::SelfModificationPipeline::new(repo_root);
            pipeline.find_gaps(spec_content)
        }
    };

    if gaps.is_empty() {
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: "No implementation gaps found. The capability may already exist.".into(),
            css_class: "message hydra".into(),
        });
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return None;
    }

    let gap_summary = gaps.iter()
        .map(|g| format!("- {}", g.description))
        .collect::<Vec<_>>()
        .join("\n");

    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: format!("Found **{}** gap(s):\n{}\n\nGenerating patches...", gaps.len(), gap_summary),
        css_class: "message hydra".into(),
    });

    Some((gaps, gap_summary))
}

/// Generate initial patches (Forge first, LLM fallback). Returns None on failure.
async fn generate_initial_patches(
    sisters_handle: &Option<SistersHandle>,
    spec_content: &str,
    gaps: &[hydra_kernel::self_modify::SpecGap],
    llm_config: &hydra_model::LlmConfig,
    repo_root: &std::path::Path,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Option<Vec<hydra_kernel::self_modify::Patch>> {
    let forge_patch_result = try_forge_generate(sisters_handle, spec_content, gaps).await;
    match hydra_kernel::self_modify_llm::generate_patches(
        gaps, spec_content, forge_patch_result, llm_config, repo_root,
    ).await {
        Ok(p) => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: format!("Generated **{}** patch(es). Applying with checkpoint...", p.len()),
                css_class: "message hydra".into(),
            });
            Some(p)
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: format!("Patch generation failed: {}", e),
                css_class: "message hydra error".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            None
        }
    }
}

/// Apply patches with agentic retry loop (up to 3 attempts).
/// On compile failure, feeds errors back to the LLM for correction.
async fn apply_with_retry(
    initial_patches: Vec<hydra_kernel::self_modify::Patch>,
    gaps: Vec<hydra_kernel::self_modify::SpecGap>,
    spec_content: &str,
    llm_config: &hydra_model::LlmConfig,
    repo_root: &std::path::Path,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let max_retries: u8 = 3;
    let mut current_patches = initial_patches;
    let mut attempt: u8 = 0;

    loop {
        attempt += 1;
        let attempt_label = if attempt == 1 { String::new() } else { format!(" (attempt {})", attempt) };

        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: format!("Applying **{}** patch(es){}...", current_patches.len(), attempt_label),
            css_class: "message hydra thinking".into(),
        });

        let pipeline = hydra_kernel::self_modify::SelfModificationPipeline::new(repo_root);
        let result = pipeline.run_from_gaps(gaps.clone(), current_patches.clone());

        match result {
            hydra_kernel::self_modify::ModResult::CompileFailed { ref error, .. } if attempt < max_retries => {
                let truncated_err = &error[..error.len().min(500)];
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: format!(
                        "Compile failed (attempt {}). Feeding errors back to LLM...\n```\n{}\n```",
                        attempt, truncated_err
                    ),
                    css_class: "message hydra thinking".into(),
                });

                match hydra_kernel::self_modify_llm::fix_compile_errors(
                    &current_patches, error, spec_content, llm_config, repo_root,
                ).await {
                    Ok(fixed_patches) => {
                        eprintln!(
                            "[hydra:self-impl] Retry {}: LLM returned {} corrected patches",
                            attempt, fixed_patches.len()
                        );
                        current_patches = fixed_patches;
                        continue;
                    }
                    Err(e) => {
                        let _ = tx.send(CognitiveUpdate::Message {
                            role: "hydra".into(),
                            content: format!(
                                "**Self-Implementation Failed**\n\nError correction failed: {}\nOriginal error: {}",
                                e, &error[..error.len().min(300)]
                            ),
                            css_class: "message hydra error".into(),
                        });
                        break;
                    }
                }
            }
            _ => {
                // Success, non-retryable failure, or max retries reached
                let summary = result.summary();
                let status_icon = if result.is_success() { "pass" } else { "warn" };
                let retry_note = if attempt > 1 {
                    format!(" (after {} attempts)", attempt)
                } else {
                    String::new()
                };

                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: format!(
                        "**Self-Implementation Report{}**\n\n[{}] {}\n\nRun `cargo check` to verify.",
                        retry_note, status_icon, summary
                    ),
                    css_class: "message hydra".into(),
                });
                break;
            }
        }
    }
}

/// Request human approval for self-modification. Returns true if approved.
async fn request_approval(
    text: &str,
    config: &CognitiveLoopConfig,
    approval_manager: &Option<Arc<ApprovalManager>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if let Some(ref mgr) = approval_manager {
        let (req, rx) = mgr.request_approval(
            &config.task_id, text, None, 0.0,
            "Self-modification: Hydra will modify its own codebase",
        );
        let _ = tx.send(CognitiveUpdate::AwaitApproval {
            approval_id: Some(req.id.clone()),
            risk_level: "high".to_string(),
            action: text.to_string(),
            description: "Self-modification requested. Hydra will analyze gaps and apply patches.".into(),
            challenge_phrase: Some(crate::cognitive::decide::ChallengePhraseGate::new(text).phrase),
        });
        match mgr.wait_for_approval(&req.id, rx).await {
            Ok(ApprovalDecision::Approved | ApprovalDecision::Modified { .. }) => {
                let _ = tx.send(CognitiveUpdate::Phase("SelfImplement — approved".into()));
                true
            }
            Ok(ApprovalDecision::Denied { reason }) => {
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: format!("Self-modification denied: {}", reason),
                    css_class: "message hydra error".into(),
                });
                false
            }
            Err(_) => {
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: "Self-modification cancelled — approval timed out.".into(),
                    css_class: "message hydra error".into(),
                });
                false
            }
        }
    } else {
        let _ = tx.send(CognitiveUpdate::AwaitApproval {
            approval_id: None,
            risk_level: "high".to_string(),
            action: text.to_string(),
            description: "Self-modification (dev mode — no approval manager).".into(),
            challenge_phrase: None,
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        true
    }
}

/// Try calling Forge sister for gap analysis. Returns None if Forge unavailable.
async fn try_forge_analyze(
    sisters: &Option<SistersHandle>,
    spec: &str,
) -> Option<Result<serde_json::Value, String>> {
    let forge = sisters.as_ref()?.forge.as_ref()?;
    Some(forge.call_tool("forge_blueprint_create", serde_json::json!({
        "spec": spec,
        "mode": "gap_analysis",
    })).await)
}

/// Try calling Forge sister for patch generation. Returns None if Forge unavailable.
async fn try_forge_generate(
    sisters: &Option<SistersHandle>,
    spec: &str,
    gaps: &[hydra_kernel::self_modify::SpecGap],
) -> Option<Result<serde_json::Value, String>> {
    let forge = sisters.as_ref()?.forge.as_ref()?;
    let gaps_json = serde_json::to_value(gaps).ok()?;
    Some(forge.call_tool("forge_generate_code", serde_json::json!({
        "spec": spec,
        "gaps": gaps_json,
    })).await)
}
