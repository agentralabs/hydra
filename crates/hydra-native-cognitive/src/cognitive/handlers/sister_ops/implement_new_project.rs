//! Phased project creation — builds sisters incrementally with full visibility.
//! Small phases, each compiles before the next, checkpoint after each, resume on interruption.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::sisters::SistersHandle;
use hydra_runtime::approval::{ApprovalDecision, ApprovalManager};
use super::super::super::loop_runner::{CognitiveLoopConfig, CognitiveUpdate};

use hydra_kernel::project_creation::{
    self, ProjectConfig, scaffold_workspace,
    run_cargo_check_project, apply_project_patch,
};
use hydra_kernel::project_creation_phases::{BuildPlan, PhaseType, PhaseStatus};
use hydra_kernel::project_creation_llm;

/// Entry point — detected by implement_diagnose when spec describes a new sister.
pub(crate) async fn handle_new_project_implement(
    spec_content: &str,
    config: &ProjectConfig,
    loop_config: &CognitiveLoopConfig,
    _sisters_handle: &Option<SistersHandle>,
    approval_manager: &Option<Arc<ApprovalManager>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let _ = tx.send(CognitiveUpdate::Phase("ProjectCreation".into()));

    let action = format!("Build new sister: {} at {}", config.name, config.target_dir.display());
    if !request_approval(&action, loop_config, approval_manager, tx).await {
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return true;
    }
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    // Check for existing checkpoint (resume support)
    let mut plan = if let Some(existing) = BuildPlan::load_checkpoint(&config.target_dir) {
        progress(tx, &format!(
            "**Resuming build**: {} ({})", existing.project_name, existing.progress_summary()
        ));
        existing
    } else {
        let plan = BuildPlan::from_config(config);
        progress(tx, &format!(
            "**Build plan**: {} phases for {}", plan.phases.len(), config.name
        ));
        plan
    };

    let llm_config = hydra_model::LlmConfig::from_env_with_overlay(
        &loop_config.anthropic_key,
        &loop_config.openai_key,
        loop_config.anthropic_oauth_token.as_deref(),
    );

    execute_phased_build(&mut plan, spec_content, config, &llm_config, tx).await;
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

/// Execute the build plan phase by phase with full visibility.
async fn execute_phased_build(
    plan: &mut BuildPlan,
    spec: &str,
    config: &ProjectConfig,
    llm_config: &hydra_model::LlmConfig,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let total = plan.phases.len();

    while let Some(phase) = plan.next_phase() {
        let idx = phase.index;
        let name = phase.name.clone();
        let ptype = phase.phase_type.clone();

        progress(tx, &format!("**Phase {}/{}**: {}", idx + 1, total, name));

        match ptype {
            PhaseType::Scaffold => {
                run_scaffold_phase(plan, config, tx).await;
            }
            PhaseType::CoreTypes | PhaseType::ToolImpl(_) | PhaseType::IntegrationTest => {
                run_llm_phase(plan, idx, spec, config, llm_config, tx).await;
            }
        }

        // Checkpoint after every phase
        match plan.save_checkpoint() {
            Ok(_) => detail(tx, "Checkpoint saved"),
            Err(e) => detail(tx, &format!("Checkpoint failed: {}", e)),
        }

        // Rate limit pause between LLM calls
        if plan.phases.get(idx).map(|p| p.requires_llm).unwrap_or(false)
            && matches!(plan.phases.get(idx).map(|p| &p.status), Some(PhaseStatus::Compiled))
        {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    // Final report
    report_final(plan, tx);
}

/// Phase 0: Scaffold workspace + short-circuit if already complete.
async fn run_scaffold_phase(
    plan: &mut BuildPlan,
    config: &ProjectConfig,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    detail(tx, &format!("Scaffolding at `{}`...", config.target_dir.display()));

    let project_dir = match scaffold_workspace(config) {
        Ok(dir) => {
            // Show what was created
            show_scaffold_files(&dir, &config.key, tx);
            dir
        }
        Err(e) => {
            plan.mark_failed(0, e.clone());
            detail(tx, &format!("Scaffold failed: {}", e));
            return;
        }
    };

    // Compile check
    match run_cargo_check_project(&project_dir) {
        Ok(_) => detail(tx, "cargo check: **PASS**"),
        Err(e) => {
            detail(tx, &format!("cargo check: **FAIL**\n```\n{}\n```", &e[..e.len().min(300)]));
            plan.mark_compiled(0); // Continue — LLM phases will fix
            return;
        }
    }

    // Test check — if tests pass, skip ALL remaining phases
    match project_creation::run_cargo_test_project(&project_dir) {
        Ok(output) => {
            let test_lines: Vec<&str> = output.lines()
                .filter(|l| l.contains("test result") || l.starts_with("test "))
                .take(10).collect();
            detail(tx, &format!("cargo test: **PASS**\n{}", test_lines.join("\n")));

            // SHORT-CIRCUIT: scaffold handles everything
            plan.mark_compiled(0);
            plan.skip_all_remaining();
            progress(tx, &format!(
                "Scaffold is complete — all {} tools working. Skipping LLM phases.",
                config.tools.len()
            ));
        }
        Err(e) => {
            detail(tx, &format!(
                "cargo test: **FAIL** (LLM phases will enhance)\n```\n{}\n```",
                &e[..e.len().min(200)]
            ));
            plan.mark_compiled(0);
        }
    }
}

/// Show what files the scaffold created.
fn show_scaffold_files(dir: &std::path::Path, key: &str, tx: &mpsc::UnboundedSender<CognitiveUpdate>) {
    let suffixes = ["", "-mcp", "-cli", "-ffi"];
    let mut listing = String::from("Files created:\n");
    for suffix in &suffixes {
        let crate_dir = dir.join(format!("crates/agentic-{}{}", key, suffix));
        if let Ok(entries) = walkdir(&crate_dir) {
            for path in entries {
                let rel = path.strip_prefix(dir).unwrap_or(&path);
                let lines = std::fs::read_to_string(&path)
                    .map(|c| c.lines().count()).unwrap_or(0);
                listing.push_str(&format!("  + {} ({} lines)\n", rel.display(), lines));
            }
        }
    }
    // Also show workspace Cargo.toml
    let wc = dir.join("Cargo.toml");
    if wc.exists() { listing.push_str("  + Cargo.toml\n"); }
    detail(tx, &listing);
}

fn walkdir(dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    if !dir.is_dir() { return Ok(files); }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() { files.extend(walkdir(&path)?); }
        else { files.push(path); }
    }
    Ok(files)
}

/// Execute a single LLM phase with full visibility.
async fn run_llm_phase(
    plan: &mut BuildPlan,
    idx: usize,
    spec: &str,
    config: &ProjectConfig,
    llm_config: &hydra_model::LlmConfig,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let phase = &plan.phases[idx];
    let completed_ctx = plan.gather_completed_context();
    let max_fixes: u8 = 3;

    // Show what we're about to do
    detail(tx, &format!("Target: `{}`", phase.target_file));

    // Read file before modification (for diff summary)
    let before_content = std::fs::read_to_string(
        plan.project_dir.join(&phase.target_file)
    ).unwrap_or_default();
    let before_lines = before_content.lines().count();

    // Generate code
    detail(tx, "Generating code...");
    let patch = match project_creation_llm::generate_phase_code(
        phase, spec, config, llm_config, &plan.project_dir, &completed_ctx,
    ).await {
        Ok(p) => p,
        Err(e) => {
            plan.mark_failed(idx, e.clone());
            detail(tx, &format!("LLM error: {}", &e[..e.len().min(200)]));
            return;
        }
    };

    // Apply + compile loop
    let mut code = patch.diff_content.clone();
    let target = patch.target_file.clone();

    for attempt in 0..=max_fixes {
        // Apply (overwrite)
        let apply = hydra_kernel::self_modify::Patch {
            target_file: target.clone(),
            diff_content: code.clone(),
            description: patch.description.clone(),
            gap: patch.gap.clone(),
            touches_critical: false,
        };
        if let Err(e) = apply_project_patch(&plan.project_dir, &apply) {
            plan.mark_failed(idx, e.clone());
            detail(tx, &format!("Apply failed: {}", e));
            return;
        }

        // Show what changed
        let after_lines = code.lines().count();
        let diff_summary = summarize_change(&before_content, &code);
        if attempt == 0 {
            detail(tx, &format!(
                "Modified: `{}` ({} → {} lines)\n{}",
                target, before_lines, after_lines, diff_summary
            ));
        }

        // Compile
        match run_cargo_check_project(&plan.project_dir) {
            Ok(_) => {
                detail(tx, "cargo check: **PASS**");
                plan.mark_compiled(idx);
                return;
            }
            Err(err) if attempt < max_fixes => {
                detail(tx, &format!(
                    "cargo check: **FAIL** (fix {}/{})\n```\n{}\n```",
                    attempt + 1, max_fixes, &err[..err.len().min(200)]
                ));
                // Fix this specific file
                let phase_ref = &plan.phases[idx];
                match project_creation_llm::fix_phase_code(
                    &code, &err, phase_ref, config, llm_config, &completed_ctx,
                ).await {
                    Ok(fixed) => {
                        let fix_diff = summarize_change(&code, &fixed.diff_content);
                        detail(tx, &format!("Fix applied:\n{}", fix_diff));
                        code = fixed.diff_content;
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                    Err(e) => {
                        plan.mark_failed(idx, format!("Fix failed: {}", e));
                        detail(tx, &format!("Could not fix: {}", &e[..e.len().min(200)]));
                        return;
                    }
                }
            }
            Err(err) => {
                plan.mark_failed(idx, err.clone());
                detail(tx, &format!(
                    "Phase failed after {} fix attempts.\n```\n{}\n```",
                    max_fixes, &err[..err.len().min(200)]
                ));
                return;
            }
        }
    }
}

/// Summarize what changed between two versions (not full dump).
fn summarize_change(before: &str, after: &str) -> String {
    let mut added = Vec::new();
    let before_set: std::collections::HashSet<&str> = before.lines().collect();
    for line in after.lines() {
        if !before_set.contains(line) && !line.trim().is_empty() {
            // Show structural lines (fn, struct, impl, use, pub, mod)
            let trimmed = line.trim();
            if trimmed.starts_with("pub ") || trimmed.starts_with("fn ")
                || trimmed.starts_with("struct ") || trimmed.starts_with("impl ")
                || trimmed.starts_with("use ") || trimmed.starts_with("mod ")
                || trimmed.starts_with("#[")
            {
                added.push(format!("  + {}", trimmed));
            }
        }
    }
    if added.is_empty() {
        "  (minor changes)".into()
    } else {
        added.truncate(10);
        added.join("\n")
    }
}

/// Final build report.
fn report_final(plan: &BuildPlan, tx: &mpsc::UnboundedSender<CognitiveUpdate>) {
    let compiled = plan.phases.iter()
        .filter(|p| matches!(p.status, PhaseStatus::Compiled)).count();
    let skipped = plan.phases.iter()
        .filter(|p| matches!(p.status, PhaseStatus::Skipped)).count();
    let failed = plan.phases.iter()
        .filter(|p| matches!(p.status, PhaseStatus::Failed(_))).count();

    let status = if failed == 0 { "pass" } else { "warn" };

    // Run final test
    let test_result = match project_creation::run_cargo_test_project(&plan.project_dir) {
        Ok(output) => {
            let lines: Vec<&str> = output.lines()
                .filter(|l| l.contains("test result") || l.starts_with("test "))
                .take(10).collect();
            format!("Tests:\n{}", lines.join("\n"))
        }
        Err(e) => format!("Tests failed:\n```\n{}\n```", &e[..e.len().min(300)]),
    };

    let report = format!(
        "**Project Creation Report**\n\n\
         [{}] {} compiled, {} skipped, {} failed\n\n\
         {}\n\n\
         Project at: `{}`",
        status, compiled, skipped, failed,
        test_result, plan.project_dir.display()
    );

    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(), content: report, css_class: "message hydra".into(),
    });
}

fn progress(tx: &mpsc::UnboundedSender<CognitiveUpdate>, msg: &str) {
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(), content: msg.to_string(),
        css_class: "message hydra".into(),
    });
}

fn detail(tx: &mpsc::UnboundedSender<CognitiveUpdate>, msg: &str) {
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(), content: msg.to_string(),
        css_class: "message hydra thinking".into(),
    });
}

async fn request_approval(
    action: &str, config: &CognitiveLoopConfig,
    mgr: &Option<Arc<ApprovalManager>>, tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let challenge = crate::cognitive::decide::ChallengePhraseGate::new(action).phrase;
    if let Some(ref m) = mgr {
        let (req, rx) = m.request_approval(
            &config.task_id, action, None, 0.0, "New project creation",
        );
        let _ = tx.send(CognitiveUpdate::AwaitApproval {
            approval_id: Some(req.id.clone()), risk_level: "high".into(),
            action: action.into(), description: "New project creation requested.".into(),
            challenge_phrase: Some(challenge),
        });
        matches!(m.wait_for_approval(&req.id, rx).await,
            Ok(ApprovalDecision::Approved | ApprovalDecision::Modified { .. }))
    } else {
        let _ = tx.send(CognitiveUpdate::AwaitApproval {
            approval_id: None, risk_level: "high".into(), action: action.into(),
            description: "New project creation (dev mode).".into(), challenge_phrase: None,
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        true
    }
}