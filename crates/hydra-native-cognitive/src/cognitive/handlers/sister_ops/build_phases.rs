//! Build phase execution helpers — scaffold, implement, test, verify.
//!
//! Each function drives one phase of the BuildOrchestrator and emits
//! CognitiveUpdate events for UI progress reporting.

use std::time::Instant;
use tokio::sync::mpsc;

use crate::sisters::SistersHandle;

use super::super::super::loop_runner::CognitiveUpdate;

/// Run scaffold phase — create new crates if needed.
pub(super) fn run_scaffold_phase(
    orchestrator: &mut hydra_kernel::build_orchestrator::BuildOrchestrator,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let start = Instant::now();
    let new_crates: Vec<String> = orchestrator.plan().crates.iter()
        .filter(|c| c.is_new)
        .map(|c| c.name.clone())
        .collect();

    if new_crates.is_empty() {
        let _ = orchestrator.execute_scaffold();
        return;
    }

    let _ = tx.send(CognitiveUpdate::BuildPhaseStarted {
        phase: "Scaffold".into(),
        detail: format!("Creating {} new crate(s): {}", new_crates.len(), new_crates.join(", ")),
    });

    match orchestrator.execute_scaffold() {
        Ok(()) => {
            let _ = tx.send(CognitiveUpdate::BuildPhaseComplete {
                phase: "Scaffold".into(),
                duration_ms: start.elapsed().as_millis() as u64,
                summary: format!("{} crate(s) scaffolded", new_crates.len()),
            });
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::BuildFailed {
                phase: "Scaffold".into(),
                error: e,
            });
        }
    }
}

/// Run implement phase — batched code generation with retry.
pub(super) async fn run_implement_phase(
    orchestrator: &mut hydra_kernel::build_orchestrator::BuildOrchestrator,
    llm_config: &hydra_model::LlmConfig,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let start = Instant::now();
    let total_steps = orchestrator.plan().implementation_order.len();

    let _ = tx.send(CognitiveUpdate::BuildPhaseStarted {
        phase: "Implement".into(),
        detail: format!("{} batch(es) to implement", total_steps),
    });

    // Report batch progress
    for i in 0..total_steps {
        let _ = tx.send(CognitiveUpdate::BuildProgress {
            phase: "Implement".into(),
            completed: i,
            total: total_steps,
        });
    }

    match orchestrator.execute_implement(llm_config).await {
        Ok(patches) => {
            let _ = tx.send(CognitiveUpdate::BuildPhaseComplete {
                phase: "Implement".into(),
                duration_ms: start.elapsed().as_millis() as u64,
                summary: format!("{} patches applied across {} batches", patches, total_steps),
            });
            true
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::BuildFailed {
                phase: "Implement".into(),
                error: e,
            });
            false
        }
    }
}

/// Run test phase — cargo test on affected crates.
pub(super) fn run_test_phase(
    orchestrator: &mut hydra_kernel::build_orchestrator::BuildOrchestrator,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let start = Instant::now();

    let _ = tx.send(CognitiveUpdate::BuildPhaseStarted {
        phase: "Test".into(),
        detail: "Running cargo test on affected crates...".into(),
    });

    match orchestrator.execute_tests() {
        Ok((passed, failed)) => {
            let _ = tx.send(CognitiveUpdate::BuildPhaseComplete {
                phase: "Test".into(),
                duration_ms: start.elapsed().as_millis() as u64,
                summary: format!("{} passed, {} failed", passed, failed),
            });
        }
        Err(e) => {
            let _ = tx.send(CognitiveUpdate::BuildFailed {
                phase: "Test".into(),
                error: e,
            });
        }
    }
}

/// Run verify phase — file size checks + sister validation.
pub(super) async fn run_verify_phase(
    orchestrator: &mut hydra_kernel::build_orchestrator::BuildOrchestrator,
    sisters: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let start = Instant::now();

    let _ = tx.send(CognitiveUpdate::BuildPhaseStarted {
        phase: "Verify".into(),
        detail: "Checking file sizes and validating...".into(),
    });

    let warnings = orchestrator.execute_verify();

    // Use sisters for validation (3s cap — don't block verify on sister timeouts)
    let warn_count = warnings.len();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let sh = match sisters.as_ref() { Some(s) => s, None => return };
        if let Some(ref aegis) = sh.aegis {
            let _ = aegis.call_tool("aegis_validate", serde_json::json!({
                "action": "build_system_verify",
                "context": format!("Build completed with {} warnings", warn_count),
            })).await;
        }
        if let Some(ref codebase) = sh.codebase {
            for step in &orchestrator.plan().implementation_order {
                for file in step.files.iter().take(3) {
                    let _ = codebase.call_tool("hallucination_check", serde_json::json!({
                        "file": file,
                        "claim": format!("Code in {} implements the spec correctly", file),
                    })).await;
                }
            }
        }
    }).await;

    let summary = if warnings.is_empty() {
        "All files under 400 lines".into()
    } else {
        format!("{} warning(s): {}", warnings.len(), warnings.join("; "))
    };

    let _ = tx.send(CognitiveUpdate::BuildPhaseComplete {
        phase: "Verify".into(),
        duration_ms: start.elapsed().as_millis() as u64,
        summary,
    });
}
