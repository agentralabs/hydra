//! Self-repair and omniscience scan handlers.

use tokio::sync::mpsc;

use crate::sisters::SistersHandle;
use hydra_native_state::utils::safe_truncate;

use super::super::super::loop_runner::CognitiveUpdate;
use super::super::super::intent_router::{IntentCategory, ClassifiedIntent};

/// Handle self-repair intent — detect "fix yourself" and run repair loop.
pub(crate) async fn handle_self_repair(
    text: &str,
    intent: &ClassifiedIntent,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if intent.category != IntentCategory::SelfRepair {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Self-Repair".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    let repo_root = std::env::current_dir().unwrap_or_default();
    let engine = crate::cognitive::self_repair::SelfRepairEngine::new(&repo_root);

    // Find the best repair_spec for this complaint, or run diagnostics
    if let Some(spec_name) = crate::cognitive::self_repair::find_spec_for_complaint(text) {
        let spec_path = repo_root.join("repair-specs").join(spec_name);
        if spec_path.exists() {
            if let Ok(spec) = engine.load_spec(&spec_path) {
                let _ = tx.send(CognitiveUpdate::RepairStarted {
                    spec: spec_name.to_string(),
                    task: spec.task.clone(),
                });

                // Run checks only (don't auto-invoke Claude from within the loop)
                let (all_pass, checks) = engine.run_all_checks(&spec).await;
                let passed = checks.iter().filter(|c| c.passed).count();

                for c in &checks {
                    let _ = tx.send(CognitiveUpdate::RepairCheckResult {
                        name: c.name.clone(),
                        passed: c.passed,
                    });
                }

                let status = if all_pass { "passing" } else { "needs_repair" };
                let _ = tx.send(CognitiveUpdate::RepairCompleted {
                    task: spec.task.clone(),
                    status: status.to_string(),
                    iterations: 0,
                });

                let msg = if all_pass {
                    format!("Self-diagnosis complete: **{}** — all {} checks passing. No repair needed.", spec.task, checks.len())
                } else {
                    let failures: Vec<String> = checks.iter()
                        .filter(|c| !c.passed)
                        .map(|c| format!("- {} *({})*", c.name, safe_truncate(&c.output, 80)))
                        .collect();
                    format!(
                        "Self-diagnosis: **{}** — {}/{} checks passing.\n\nFailing checks:\n{}\n\nRun `./scripts/hydra-self-repair.sh repair-specs/{}` to auto-repair.",
                        spec.task, passed, checks.len(), failures.join("\n"), spec_name
                    )
                };
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: msg,
                    css_class: "message hydra self-repair".into(),
                });
                let _ = tx.send(CognitiveUpdate::ResetIdle);
                return true;
            }
        }
    }

    // No specific spec found — run full diagnostics
    let status = engine.status().await;
    let total = status.len();
    let passing = status.iter().filter(|(_, _, p, t)| p == t).count();

    let summary: String = status.iter()
        .map(|(file, task, passed, total)| {
            let icon = if passed == total { "✅" } else { "⚠️" };
            format!("{} **{}** ({}/{} checks) — {}", icon, file, passed, total, task)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let msg = format!(
        "Self-repair diagnostics: **{}/{}** specs fully passing.\n\n{}\n\nRun `./scripts/hydra-repair-all.sh` to repair all failing specs.",
        passing, total, summary
    );
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: msg,
        css_class: "message hydra self-repair".into(),
    });
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

/// Handle omniscience scan — full semantic self-repair via Codebase + Forge + Aegis.
pub(crate) async fn handle_omniscience_scan(
    intent: &ClassifiedIntent,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if intent.category != IntentCategory::SelfScan {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Omniscience".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    let repo_root = std::env::current_dir().unwrap_or_default();
    let omni = crate::cognitive::omniscience::OmniscienceEngine::new(&repo_root);

    // Create a channel for omniscience updates
    let (omni_tx, mut omni_rx) = mpsc::unbounded_channel();

    // Clone tx for the forwarding task
    let tx2 = tx.clone();
    let forward_task = tokio::spawn(async move {
        while let Some(update) = omni_rx.recv().await {
            match update {
                crate::cognitive::omniscience::OmniscienceUpdate::CodebaseAnalyzing { phase } => {
                    let _ = tx2.send(CognitiveUpdate::OmniscienceAnalyzing { phase });
                }
                crate::cognitive::omniscience::OmniscienceUpdate::GapFound(gap) => {
                    let _ = tx2.send(CognitiveUpdate::OmniscienceGapFound {
                        description: gap.description,
                        severity: gap.severity,
                        category: gap.category,
                    });
                }
                crate::cognitive::omniscience::OmniscienceUpdate::SpecGenerated { spec_name, task } => {
                    let _ = tx2.send(CognitiveUpdate::OmniscienceSpecGenerated { spec_name, task });
                }
                crate::cognitive::omniscience::OmniscienceUpdate::AegisValidation { spec_name, safe, recommendation } => {
                    let _ = tx2.send(CognitiveUpdate::OmniscienceValidation { spec_name, safe, recommendation });
                }
                crate::cognitive::omniscience::OmniscienceUpdate::ScanComplete(scan) => {
                    let _ = tx2.send(CognitiveUpdate::OmniscienceScanComplete {
                        gaps_found: scan.gaps.len(),
                        specs_generated: scan.generated_specs.len(),
                        health_score: scan.code_health_score,
                    });
                }
            }
        }
    });

    // Run the omniscience loop (needs Sisters)
    if let Some(ref sh) = sisters_handle {
        let scan = omni.run_omniscience_loop(sh, Some(&omni_tx)).await;
        drop(omni_tx);
        let _ = forward_task.await;

        let health_pct = (scan.code_health_score * 100.0) as u32;
        let repos_scanned = scan.repo_scans.len();
        let repos_healthy = scan.repo_scans.iter()
            .filter(|r| r.health_score >= 0.9)
            .count();

        // Per-repo summary
        let repo_summary: String = scan.repo_scans.iter()
            .map(|r| {
                let h = (r.health_score * 100.0) as u32;
                let icon = if r.health_score >= 0.9 { "✅" } else if r.health_score >= 0.7 { "⚠️" } else { "❌" };
                format!("{} **{}** — {}% health ({} files, {} gaps, {} specs)",
                    icon, r.repo, h, r.files_analyzed, r.gaps.len(), r.generated_specs.len())
            })
            .collect::<Vec<_>>()
            .join("\n");

        let msg = format!(
            "**Omniscience Scan Complete** — {}/{} repos healthy\n\n\
             | Metric | Value |\n\
             |--------|-------|\n\
             | Repos scanned | **{}** |\n\
             | Total files | **{}** |\n\
             | Overall health | **{}%** |\n\
             | Total gaps | **{}** |\n\
             | Specs generated | **{}** |\n\n\
             ### Per-Repo Health\n{}\n\n\
             {}\n\n\
             Run `./scripts/hydra-repair-all.sh` to auto-repair generated specs.",
            repos_healthy, repos_scanned,
            repos_scanned,
            scan.total_files_analyzed,
            health_pct,
            scan.gaps.len(),
            scan.generated_specs.len(),
            repo_summary,
            if scan.gaps.is_empty() {
                "No gaps detected — all codebases are healthy.".to_string()
            } else {
                let gap_summary: String = scan.gaps.iter().take(15)
                    .map(|g| format!("- [{}|{}] {} — {}", g.repo, g.severity, g.category, g.description))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("### Top Gaps\n{}", gap_summary)
            }
        );
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: msg,
            css_class: "message hydra omniscience".into(),
        });
    } else {
        drop(omni_tx);
        let _ = forward_task.await;
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: "Omniscience loop requires Sisters to be connected (Codebase + Forge + Aegis).".into(),
            css_class: "message hydra error".into(),
        });
    }

    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}
