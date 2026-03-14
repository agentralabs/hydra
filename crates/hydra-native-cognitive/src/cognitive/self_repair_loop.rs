//! Self-Repair Loop — repair orchestration and Claude Code invocation.
//!
//! Contains the repair loop that iterates over acceptance checks, invokes
//! Claude Code for fixes, and reports progress. Separated from the core
//! types and check evaluation in `self_repair.rs`.

use std::path::Path;
use std::process::Stdio;
use std::time::Instant;

use tokio::sync::mpsc;

use super::self_repair::{
    CheckResult, RepairResult, RepairStatus, RepairUpdate, SelfRepairEngine,
};

impl SelfRepairEngine {
    /// Run the full self-repair loop for a spec, sending progress updates.
    pub async fn repair(
        &self,
        spec_path: &Path,
        tx: Option<&mpsc::UnboundedSender<RepairUpdate>>,
    ) -> RepairResult {
        let start = Instant::now();
        let spec_name = spec_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let spec = match self.load_spec(spec_path) {
            Ok(s) => s,
            Err(e) => {
                return RepairResult {
                    spec_file: spec_name,
                    task: format!("Failed to load: {}", e),
                    status: RepairStatus::Failed,
                    iterations: 0,
                    checks: vec![],
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }
        };

        if let Some(tx) = tx {
            let _ = tx.send(RepairUpdate::Started {
                spec: spec_name.clone(),
                task: spec.task.clone(),
                max_iterations: spec.max_iterations,
            });
        }

        for iteration in 1..=spec.max_iterations {
            if let Some(tx) = tx {
                let _ = tx.send(RepairUpdate::IterationStart { iteration });
            }

            // Run acceptance checks
            let (all_pass, results) = self.run_all_checks(&spec).await;
            let passed_count = results.iter().filter(|r| r.passed).count();

            if let Some(tx) = tx {
                for r in &results {
                    let _ = tx.send(RepairUpdate::CheckResult {
                        name: r.name.clone(),
                        passed: r.passed,
                        output: r.output.clone(),
                    });
                }
                let _ = tx.send(RepairUpdate::IterationComplete {
                    iteration,
                    passed: passed_count,
                    total: results.len(),
                });
            }

            if all_pass {
                let result = RepairResult {
                    spec_file: spec_name,
                    task: spec.task.clone(),
                    status: if iteration == 1 { RepairStatus::AlreadyPassing } else { RepairStatus::Success },
                    iterations: iteration,
                    checks: results,
                    duration_ms: start.elapsed().as_millis() as u64,
                };
                if let Some(tx) = tx {
                    let _ = tx.send(RepairUpdate::Completed(result.clone()));
                }
                return result;
            }

            // Build failure context for Claude Code
            let failures: Vec<&CheckResult> = results.iter().filter(|r| !r.passed).collect();
            let failure_log: String = failures.iter()
                .map(|f| format!("FAILED: {}\n  Output: {}", f.name, f.output))
                .collect::<Vec<_>>()
                .join("\n\n");

            let prompt = if iteration == 1 {
                spec.instructions_for_claude_code.clone()
            } else {
                format!(
                    "PREVIOUS ATTEMPT FAILED (iteration {}). Specific failures:\n\n{}\n\n\
                     Fix ALL failing checks. The acceptance criteria are non-negotiable.\n\n\
                     Original instructions:\n{}",
                    iteration - 1, failure_log, spec.instructions_for_claude_code
                )
            };

            if let Some(tx) = tx {
                let _ = tx.send(RepairUpdate::ClaudeInvoked { iteration });
            }

            // Invoke Claude Code
            let _claude_output = self.invoke_claude(&prompt).await;
        }

        // Max iterations reached — escalate
        let (_, final_checks) = self.run_all_checks(&spec).await;
        let passed_count = final_checks.iter().filter(|r| r.passed).count();

        let result = RepairResult {
            spec_file: spec_name,
            task: spec.task.clone(),
            status: if passed_count == final_checks.len() { RepairStatus::Success } else { RepairStatus::Escalated },
            iterations: spec.max_iterations,
            checks: final_checks,
            duration_ms: start.elapsed().as_millis() as u64,
        };

        if let Some(tx) = tx {
            let _ = tx.send(RepairUpdate::Completed(result.clone()));
        }

        result
    }

    /// Invoke Claude Code CLI with repair instructions.
    async fn invoke_claude(&self, prompt: &str) -> String {
        let result = tokio::process::Command::new("claude")
            .args(["--dangerously-skip-permissions", "--print", "--output-format", "text"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&self.repo_root)
            .spawn();

        match result {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    use tokio::io::AsyncWriteExt;
                    if let Err(e) = stdin.write_all(prompt.as_bytes()).await {
                        eprintln!("[hydra:self-repair] stdin write_all FAILED: {}", e);
                    }
                    if let Err(e) = stdin.shutdown().await {
                        eprintln!("[hydra:self-repair] stdin shutdown FAILED: {}", e);
                    }
                }
                match child.wait_with_output().await {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        format!("{}\n{}", stdout, stderr)
                    }
                    Err(e) => format!("Claude Code error: {}", e),
                }
            }
            Err(e) => format!("Failed to launch Claude Code: {}", e),
        }
    }

    /// Quick status check: how many specs are currently passing?
    pub async fn status(&self) -> Vec<(String, String, usize, usize)> {
        let mut results = Vec::new();
        for spec_path in self.list_specs() {
            if let Ok(spec) = self.load_spec(&spec_path) {
                let (_, checks) = self.run_all_checks(&spec).await;
                let passed = checks.iter().filter(|c| c.passed).count();
                results.push((
                    spec_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                    spec.task,
                    passed,
                    checks.len(),
                ));
            }
        }
        results
    }
}
