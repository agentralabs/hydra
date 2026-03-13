//! Build system handler — full multi-phase system builder leveraging all coding sisters.
//!
//! Flow: read spec → approve → plan (Forge) → scaffold → implement → test → verify → report.
//! Uses: Forge (blueprints, structure, skeletons), Codebase (symbol lookup, impact analysis,
//! hallucination check), Aegis (shadow validation), and LLM fallback.

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::sisters::SistersHandle;
use hydra_runtime::approval::{ApprovalDecision, ApprovalManager};

use super::super::super::loop_runner::{CognitiveLoopConfig, CognitiveUpdate};
use super::super::super::intent_router::{IntentCategory, ClassifiedIntent};
use super::build_phases::{run_scaffold_phase, run_implement_phase, run_test_phase, run_verify_phase};

/// Handle "build specs/X.md" — multi-phase system builder.
pub(crate) async fn handle_build_system(
    text: &str,
    intent: &ClassifiedIntent,
    config: &CognitiveLoopConfig,
    sisters_handle: &Option<SistersHandle>,
    approval_manager: &Option<Arc<ApprovalManager>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let lower = text.to_lowercase();
    let keyword_match = (lower.contains("build") && lower.contains(".md"))
        || lower.starts_with("build specs/")
        || lower.starts_with("build system ");

    if intent.category != IntentCategory::BuildSystem && !keyword_match {
        return false;
    }

    let has_spec = hydra_kernel::self_modify_llm::extract_spec_path(text).is_some();
    if !has_spec && !keyword_match {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("BuildSystem".into()));
    let _ = tx.send(CognitiveUpdate::IconState("needs-attention".into()));

    let repo_root = std::env::current_dir().unwrap_or_default();

    // -- Step 1: Read spec --
    let spec_content = match read_build_spec(text, &repo_root, tx) {
        Some(s) => s,
        None => return true,
    };

    // -- Step 2: Approval --
    if !request_build_approval(text, config, approval_manager, tx).await {
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return true;
    }

    let _ = tx.send(CognitiveUpdate::IconState("working".into()));
    let build_start = Instant::now();

    // -- Step 3: Build LLM config --
    let llm_config = hydra_model::LlmConfig::from_env_with_overlay(
        &config.anthropic_key,
        &config.openai_key,
        config.anthropic_oauth_token.as_deref(),
    );

    // -- Step 4: Enrich spec with sister intelligence (5s cap) --
    let _ = tx.send(CognitiveUpdate::Phase("BuildSystem — gathering context...".into()));
    let enriched_spec = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        enrich_with_sisters(sisters_handle, &spec_content, &repo_root, tx),
    ).await {
        Ok(s) => s,
        Err(_) => {
            eprintln!("[hydra:build] Sister enrichment timed out (5s), using spec + workspace only");
            let ws = hydra_kernel::self_modify_llm::gather_workspace_context(&repo_root);
            format!("{}\n\n## Workspace Structure\n{}", spec_content, ws)
        }
    };

    // -- Step 5: Generate build plan (LLM) --
    let _ = tx.send(CognitiveUpdate::BuildPhaseStarted {
        phase: "Plan".into(),
        detail: "Analyzing spec and generating build plan...".into(),
    });

    let plan = match hydra_kernel::build_planner::generate_build_plan(
        &enriched_spec, &llm_config, &repo_root,
    ).await {
        Ok(p) => {
            let _ = tx.send(CognitiveUpdate::BuildPhaseComplete {
                phase: "Plan".into(),
                duration_ms: build_start.elapsed().as_millis() as u64,
                summary: format!(
                    "{} crate(s), {} step(s), complexity: {:?}",
                    p.crates.len(), p.implementation_order.len(), p.complexity
                ),
            });
            p
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::BuildFailed {
                phase: "Plan".into(),
                error: format!("Build plan generation failed: {}", e),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            return true;
        }
    };

    // -- Step 6: Run orchestrator --
    let mut orchestrator = hydra_kernel::build_orchestrator::BuildOrchestrator::new(
        repo_root.clone(), enriched_spec.clone(), plan,
    );

    // Phase 1: Scaffold
    run_scaffold_phase(&mut orchestrator, tx);

    // Phase 2: Implement (batched with retry)
    let implement_ok = run_implement_phase(&mut orchestrator, &llm_config, tx).await;

    if implement_ok {
        // Phase 3: Test
        run_test_phase(&mut orchestrator, tx);

        // Phase 4: Verify
        run_verify_phase(&mut orchestrator, sisters_handle, tx).await;
    }

    // -- Step 7: Final report --
    let report = orchestrator.finalize();
    let duration_s = build_start.elapsed().as_secs();

    let report_text = format!(
        "**Build Complete** ({:.0}s)\n\n\
         | Metric | Value |\n|---|---|\n\
         | Crates created | {} |\n\
         | Files modified | {} |\n\
         | Patches applied | {} |\n\
         | Tests passing | {} |\n\
         | Tests failed | {} |\n\
         | Batches | {} |\n\
         | Retries used | {} |\n\n\
         Run `cargo check` and `cargo test` to verify.",
        duration_s,
        report.crates_created, report.files_modified, report.patches_applied,
        report.tests_passing, report.tests_failed,
        report.batches_completed, report.retries_used,
    );

    let _ = tx.send(CognitiveUpdate::BuildComplete { report: report_text.clone() });
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: report_text,
        css_class: "message hydra".into(),
    });
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

/// Read spec file from path in user text.
fn read_build_spec(
    text: &str,
    repo_root: &std::path::Path,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Option<String> {
    let spec_path = hydra_kernel::self_modify_llm::extract_spec_path(text)?;
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
            let _ = tx.send(CognitiveUpdate::BuildFailed {
                phase: "Read".into(),
                error: format!("Cannot read spec: {}", e),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            None
        }
    }
}

/// Request approval for build operation.
async fn request_build_approval(
    text: &str,
    config: &CognitiveLoopConfig,
    approval_manager: &Option<Arc<ApprovalManager>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if let Some(ref mgr) = approval_manager {
        let (req, rx) = mgr.request_approval(
            &config.task_id, text, None, 0.0,
            "Build System: Hydra will scaffold crates, generate code, and run tests",
        );
        let _ = tx.send(CognitiveUpdate::AwaitApproval {
            approval_id: Some(req.id.clone()),
            risk_level: "high".to_string(),
            action: text.to_string(),
            description: "Multi-phase build. Hydra will create crates, generate code, and modify the workspace.".into(),
            challenge_phrase: Some(crate::cognitive::decide::ChallengePhraseGate::new(text).phrase),
        });
        match mgr.wait_for_approval(&req.id, rx).await {
            Ok(ApprovalDecision::Approved | ApprovalDecision::Modified { .. }) => true,
            _ => false,
        }
    } else {
        let _ = tx.send(CognitiveUpdate::AwaitApproval {
            approval_id: None,
            risk_level: "high".to_string(),
            action: text.to_string(),
            description: "Build system (dev mode — no approval manager).".into(),
            challenge_phrase: None,
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        true
    }
}

/// Enrich the spec with sister intelligence before planning.
async fn enrich_with_sisters(
    sisters: &Option<SistersHandle>,
    spec: &str,
    repo_root: &std::path::Path,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> String {
    let sh = match sisters.as_ref() {
        Some(s) => s,
        None => return spec.to_string(),
    };

    let mut context_parts: Vec<String> = vec![spec.to_string()];

    // Codebase: find existing related code via concept_find
    if let Some(ref codebase) = sh.codebase {
        let title = extract_spec_title(spec);
        if let Ok(result) = codebase.call_tool("concept_find", serde_json::json!({
            "concept": title, "limit": 10,
        })).await {
            let text = crate::sisters::connection::extract_text(&result);
            if !text.is_empty() && text.len() > 10 {
                context_parts.push(format!(
                    "\n## Existing Related Code (from Codebase sister)\n{}", text
                ));
                let _ = tx.send(CognitiveUpdate::Phase("BuildSystem — queried codebase".into()));
            }
        }
    }

    // Codebase: symbol lookup for key terms in the spec
    if let Some(ref codebase) = sh.codebase {
        let keywords = extract_keywords(spec);
        for kw in keywords.iter().take(3) {
            if let Ok(result) = codebase.call_tool("symbol_lookup", serde_json::json!({
                "name": kw, "limit": 5,
            })).await {
                let text = crate::sisters::connection::extract_text(&result);
                if !text.is_empty() && text.len() > 10 {
                    context_parts.push(format!("\n## Existing symbols for '{}'\n{}", kw, text));
                }
            }
        }
    }

    // Forge: create a blueprint for structural understanding
    if let Some(ref forge) = sh.forge {
        if let Ok(result) = forge.call_tool("forge_blueprint_create", serde_json::json!({
            "intent": spec,
            "mode": "analysis",
        })).await {
            let text = crate::sisters::connection::extract_text(&result);
            if !text.is_empty() && text.len() > 10 {
                context_parts.push(format!(
                    "\n## Forge Blueprint Analysis\n{}", text
                ));
                let _ = tx.send(CognitiveUpdate::Phase("BuildSystem — Forge blueprint".into()));
            }
        }
    }

    // Forge: resolve dependencies
    if let Some(ref forge) = sh.forge {
        if let Ok(result) = forge.call_tool("forge_dependency_resolve", serde_json::json!({
            "spec": &spec[..spec.len().min(2000)],
        })).await {
            let text = crate::sisters::connection::extract_text(&result);
            if !text.is_empty() && text.len() > 10 {
                context_parts.push(format!("\n## Dependency Analysis\n{}", text));
            }
        }
    }

    // Workspace context (always available, no sister needed)
    let ws_ctx = hydra_kernel::self_modify_llm::gather_workspace_context(repo_root);
    context_parts.push(format!("\n## Workspace Structure\n{}", ws_ctx));

    context_parts.join("\n")
}

/// Extract a title from the first heading in the spec.
fn extract_spec_title(spec: &str) -> String {
    spec.lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches('#').trim().to_string())
        .unwrap_or_else(|| "system".into())
}

/// Extract key domain keywords from a spec for symbol lookup.
fn extract_keywords(spec: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for line in spec.lines().take(50) {
        let trimmed = line.trim();
        // Extract from headings and code blocks
        if trimmed.starts_with('#') || trimmed.starts_with("- ") || trimmed.contains("fn ") {
            for word in trimmed.split_whitespace() {
                let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                if clean.len() > 3 && clean.len() < 40
                    && !["the", "and", "for", "with", "from", "this", "that", "must", "should"]
                        .contains(&clean.to_lowercase().as_str())
                {
                    if !words.contains(&clean.to_string()) {
                        words.push(clean.to_string());
                    }
                }
            }
        }
    }
    words.into_iter().take(5).collect()
}
