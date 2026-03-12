//! Self-implement and sister diagnostics handlers.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::sisters::SistersHandle;
use hydra_runtime::approval::{ApprovalDecision, ApprovalManager};

use super::super::super::loop_runner::{CognitiveLoopConfig, CognitiveUpdate};
use super::super::super::intent_router::{IntentCategory, ClassifiedIntent};

/// Handle self-implement — self-modification pipeline (Phase 5, Priority 1).
///
/// Flow: read spec → approve → Forge/LLM gap analysis → Forge/LLM patch gen → apply → report.
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

    // Require either keyword match or high-confidence SelfImplement classification
    if intent.category != IntentCategory::SelfImplement && !keyword_match {
        return false;
    }

    // Guard: SelfImplement requires a spec file path. If user just says "implement that"
    // without a file, let it fall through to the normal LLM conversation.
    let has_spec_file = hydra_kernel::self_modify_llm::extract_spec_path(text).is_some();
    if !has_spec_file && !keyword_match {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("SelfImplement".into()));
    let _ = tx.send(CognitiveUpdate::IconState("needs-attention".into()));

    // ── Step 1: Read spec (file or inline text) ──
    let repo_root = std::env::current_dir().unwrap_or_default();
    let spec_content = if let Some(spec_path) = hydra_kernel::self_modify_llm::extract_spec_path(text) {
        let full_path = repo_root.join(&spec_path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: format!("Read spec: **{}** ({} bytes)", spec_path.display(), content.len()),
                    css_class: "message hydra thinking".into(),
                });
                content
            }
            Err(e) => {
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: format!("Cannot read spec file `{}`: {}", full_path.display(), e),
                    css_class: "message hydra error".into(),
                });
                let _ = tx.send(CognitiveUpdate::ResetIdle);
                return true;
            }
        }
    } else {
        // No file path found — use the raw text as inline spec
        text.to_string()
    };

    // ── Step 2: Request human approval ──
    let approved = request_approval(text, config, approval_manager, tx).await;
    if !approved {
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return true;
    }

    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    // ── Step 3: Build LLM config (sanitized keys) ──
    let llm_config = hydra_model::LlmConfig::from_env_with_overlay(
        &config.anthropic_key,
        &config.openai_key,
        config.anthropic_oauth_token.as_deref(),
    );

    // ── Step 4: Gap analysis (Forge first, LLM fallback) ──
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: "Analyzing spec for implementation gaps...".into(),
        css_class: "message hydra thinking".into(),
    });

    let forge_gap_result = try_forge_analyze(sisters_handle, &spec_content).await;
    let gaps = match hydra_kernel::self_modify_llm::analyze_spec_gaps(
        &spec_content, forge_gap_result, &llm_config,
    ).await {
        Ok(g) => g,
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: format!("Gap analysis failed: {}. Falling back to pattern matching...", e),
                css_class: "message hydra error".into(),
            });
            // Fall back to regex-based gap detection
            let pipeline = hydra_kernel::self_modify::SelfModificationPipeline::new(&repo_root);
            pipeline.find_gaps(&spec_content)
        }
    };

    if gaps.is_empty() {
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: "No implementation gaps found. The capability may already exist.".into(),
            css_class: "message hydra".into(),
        });
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return true;
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

    // ── Step 5: Patch generation (Forge first, LLM fallback) ──
    let forge_patch_result = try_forge_generate(sisters_handle, &spec_content, &gaps).await;
    let patches = match hydra_kernel::self_modify_llm::generate_patches(
        &gaps, &spec_content, forge_patch_result, &llm_config,
    ).await {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: format!("Patch generation failed: {}", e),
                css_class: "message hydra error".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            return true;
        }
    };

    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: format!("Generated **{}** patch(es). Applying with checkpoint...", patches.len()),
        css_class: "message hydra".into(),
    });

    // ── Step 6: Apply patches via pipeline ──
    let pipeline = hydra_kernel::self_modify::SelfModificationPipeline::new(&repo_root);
    let result = pipeline.run_from_gaps(gaps, patches);

    // ── Step 7: Report results ──
    let summary = result.summary();
    let status_icon = if result.is_success() { "✅" } else { "⚠️" };

    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: format!(
            "**Self-Implementation Report**\n\n{} {}\n\nRun `cargo check` to verify.",
            status_icon, summary
        ),
        css_class: "message hydra".into(),
    });

    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
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
        // Dev mode — auto-approve with notification
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

/// Handle sister diagnostics — direct sister health check (no LLM needed).
pub(crate) async fn handle_sister_diagnose(
    text: &str,
    intent: &ClassifiedIntent,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    // Skip SisterDiagnose if the user is asking about policies/rules/capabilities — let LLM handle it
    let lower_for_policy = text.to_lowercase();
    let is_policy_query = lower_for_policy.contains("policy") || lower_for_policy.contains("policies")
        || lower_for_policy.contains("rules") || lower_for_policy.contains("what does")
        || lower_for_policy.contains("capabilities") || lower_for_policy.contains("what can");
    if intent.category != IntentCategory::SisterDiagnose || is_policy_query {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Diagnostics".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    if let Some(ref sh) = sisters_handle {
        let target_sister = intent.target.clone();
        let mut report = String::new();

        // Header
        report.push_str("## Sister Diagnostics\n\n");

        // Overall status
        let connected = sh.connected_count();
        report.push_str(&format!("**{}/14 sisters connected**\n\n", connected));

        // Per-sister detail
        report.push_str("| Sister | Status | Tools |\n|--------|--------|-------|\n");
        for (name, opt) in sh.all_sisters() {
            let (status, tools) = if let Some(conn) = opt {
                ("ONLINE", conn.tools.len().to_string())
            } else {
                ("OFFLINE", "-".to_string())
            };
            let icon = if opt.is_some() { "🟢" } else { "🔴" };
            report.push_str(&format!("| {} {} | {} | {} |\n", icon, name, status, tools));
        }

        // If user asked about a specific sister, do a deeper probe
        if let Some(ref target) = target_sister {
            report.push_str(&format!("\n### Deep Probe: {}\n\n", target));
            let probe_result = match target.to_lowercase().as_str() {
                "memory" | "agenticmemory" => {
                    if let Some(mem) = &sh.memory {
                        let r = mem.call_tool("memory_longevity_stats", serde_json::json!({})).await;
                        match r {
                            Ok(v) => format!("Memory stats: {}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                            Err(e) => format!("Memory probe FAILED: {}", e),
                        }
                    } else {
                        "Memory sister is NOT connected.".to_string()
                    }
                }
                "identity" | "agenticidentity" => {
                    if let Some(id) = &sh.identity {
                        let r = id.call_tool("identity_whoami", serde_json::json!({})).await;
                        match r {
                            Ok(v) => format!("Identity probe: {}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                            Err(e) => format!("Identity probe FAILED: {}", e),
                        }
                    } else {
                        "Identity sister is NOT connected.".to_string()
                    }
                }
                "cognition" | "agenticcognition" => {
                    if let Some(cog) = &sh.cognition {
                        let r = cog.call_tool("cognition_model_query", serde_json::json!({"context": "diagnostic"})).await;
                        match r {
                            Ok(v) => format!("Cognition probe: {}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                            Err(e) => format!("Cognition probe FAILED: {}", e),
                        }
                    } else {
                        "Cognition sister is NOT connected.".to_string()
                    }
                }
                _ => {
                    // Generic: check if the named sister is connected
                    let found = sh.all_sisters().iter()
                        .find(|(n, _)| n.to_lowercase() == target.to_lowercase())
                        .map(|(_, opt)| opt.is_some());
                    match found {
                        Some(true) => format!("{} sister is connected and responsive.", target),
                        Some(false) => format!("{} sister is NOT connected. It failed to spawn at startup.", target),
                        None => format!("Unknown sister: {}. Known sisters: Memory, Identity, Codebase, Vision, Comm, Contract, Time, Planning, Cognition, Reality, Forge, Aegis, Veritas, Evolve.", target),
                    }
                }
            };
            report.push_str(&probe_result);
        }

        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: report,
            css_class: "message hydra diagnostics".into(),
        });
    } else {
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: "No sisters available — running in offline mode.".into(),
            css_class: "message hydra error".into(),
        });
    }

    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}
