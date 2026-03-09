//! Self-Repair Engine — Bootstrap Omniscience Loop.
//!
//! Reads JSON repair specs, runs acceptance checks, invokes Claude Code
//! for fixes, and loops until all checks pass. This is the bootstrap
//! version that doesn't need semantic code understanding — just PASS/FAIL.
//!
//! Full Omniscience (future): Codebase sister reads own code → Forge generates
//! fixes → Aegis validates → auto-applies. This bootstrap gets us there.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// A repair spec loaded from a JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairSpec {
    pub task: String,
    pub priority: u32,
    #[serde(default = "default_max_iter")]
    pub max_iterations: u32,
    pub description: String,
    #[serde(default)]
    pub files_to_modify: Vec<String>,
    pub acceptance_checks: Vec<AcceptanceCheck>,
    pub instructions_for_claude_code: String,
}

fn default_max_iter() -> u32 { 5 }

/// A single acceptance check — binary PASS/FAIL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCheck {
    pub name: String,
    pub check: String,
    #[serde(default)]
    pub expect: Option<String>,
    #[serde(default)]
    pub expect_minimum: Option<i64>,
    #[serde(default)]
    pub expect_maximum: Option<i64>,
}

/// Result of running a single check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub output: String,
}

/// Result of a full repair run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairResult {
    pub spec_file: String,
    pub task: String,
    pub status: RepairStatus,
    pub iterations: u32,
    pub checks: Vec<CheckResult>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RepairStatus {
    Success,
    Failed,
    Escalated,
    AlreadyPassing,
}

/// Progress updates sent during repair.
#[derive(Debug, Clone)]
pub enum RepairUpdate {
    /// Repair started for a spec.
    Started { spec: String, task: String, max_iterations: u32 },
    /// An iteration started.
    IterationStart { iteration: u32 },
    /// A check was evaluated.
    CheckResult { name: String, passed: bool, output: String },
    /// All checks evaluated for an iteration.
    IterationComplete { iteration: u32, passed: usize, total: usize },
    /// Claude Code invoked for repair.
    ClaudeInvoked { iteration: u32 },
    /// Repair completed (success or failure).
    Completed(RepairResult),
}

/// The self-repair engine.
pub struct SelfRepairEngine {
    repo_root: PathBuf,
    specs_dir: PathBuf,
}

impl SelfRepairEngine {
    pub fn new(repo_root: impl Into<PathBuf>) -> Self {
        let root = repo_root.into();
        let specs_dir = root.join("repair-specs");
        Self { repo_root: root, specs_dir }
    }

    /// Load a repair spec from a JSON file.
    pub fn load_spec(&self, path: &Path) -> Result<RepairSpec, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read spec {}: {}", path.display(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse spec {}: {}", path.display(), e))
    }

    /// List all available repair specs, sorted by filename.
    pub fn list_specs(&self) -> Vec<PathBuf> {
        let mut specs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.specs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    specs.push(path);
                }
            }
        }
        specs.sort();
        specs
    }

    /// Run a single acceptance check. Returns (passed, output).
    pub async fn run_check(&self, check: &AcceptanceCheck) -> CheckResult {
        let output = match tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&check.check)
            .current_dir(&self.repo_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
        {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                format!("{} {}", stdout.trim(), stderr.trim()).trim().to_string()
            }
            Err(e) => format!("ERROR: {}", e),
        };

        let passed = evaluate_check(check, &output);

        CheckResult {
            name: check.name.clone(),
            passed,
            output: output.chars().take(500).collect(),
        }
    }

    /// Run ALL acceptance checks for a spec. Returns (all_passed, results).
    pub async fn run_all_checks(&self, spec: &RepairSpec) -> (bool, Vec<CheckResult>) {
        let mut results = Vec::new();
        let mut all_pass = true;

        for check in &spec.acceptance_checks {
            let result = self.run_check(check).await;
            if !result.passed {
                all_pass = false;
            }
            results.push(result);
        }

        (all_pass, results)
    }

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
                    let _ = stdin.write_all(prompt.as_bytes()).await;
                    let _ = stdin.shutdown().await;
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

    /// Generate a repair spec from Codebase sister analysis (Omniscience bootstrap).
    /// This is the bridge to full Omniscience — once the Codebase sister can read
    /// Hydra's own code, it can generate specs for gaps it finds.
    pub fn generate_spec_from_analysis(
        &self,
        task: &str,
        description: &str,
        files: &[&str],
        checks: Vec<AcceptanceCheck>,
        instructions: &str,
    ) -> RepairSpec {
        RepairSpec {
            task: task.to_string(),
            priority: 5,
            max_iterations: 5,
            description: description.to_string(),
            files_to_modify: files.iter().map(|s| s.to_string()).collect(),
            acceptance_checks: checks,
            instructions_for_claude_code: instructions.to_string(),
        }
    }

    /// Save a generated spec to disk.
    pub fn save_spec(&self, spec: &RepairSpec, filename: &str) -> Result<PathBuf, String> {
        let path = self.specs_dir.join(filename);
        let content = serde_json::to_string_pretty(spec)
            .map_err(|e| format!("Failed to serialize spec: {}", e))?;
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write spec: {}", e))?;
        Ok(path)
    }
}

/// Evaluate a check result against expectations.
fn evaluate_check(check: &AcceptanceCheck, output: &str) -> bool {
    if let Some(ref expect) = check.expect {
        match expect.as_str() {
            "found" => {
                // Non-empty output from a successful command
                !output.is_empty() && !output.starts_with("ERROR:")
            }
            "not_found" => {
                output.is_empty() || output == "0"
            }
            other => {
                output.contains(other)
            }
        }
    } else if let Some(min) = check.expect_minimum {
        // Extract numbers from output, use the last one
        let nums: Vec<i64> = output.split_whitespace()
            .filter_map(|w| w.parse::<i64>().ok())
            .collect();
        nums.last().copied().unwrap_or(0) >= min
    } else if let Some(max) = check.expect_maximum {
        let nums: Vec<i64> = output.split_whitespace()
            .filter_map(|w| w.parse::<i64>().ok())
            .collect();
        nums.last().copied().unwrap_or(999999) <= max
    } else {
        // No expectation = just check non-error
        !output.starts_with("ERROR:")
    }
}

/// Detect if user input is a self-repair intent.
pub fn is_self_repair_intent(text: &str) -> bool {
    let lower = text.to_lowercase();
    let patterns = [
        "fix yourself", "self-repair", "self repair", "repair yourself",
        "fix your", "heal yourself", "fix this bug in hydra",
        "repair your memory", "repair your beliefs", "fix your memory",
        "your memory is broken", "you're not saving", "you don't remember",
        "nothing is being saved", "repair your code", "fix your code",
        "run self-repair", "run repair", "self-diagnose", "diagnose yourself",
        "check your systems", "check yourself", "run diagnostics",
        "fix your federation", "fix your beliefs", "repair spec",
    ];
    patterns.iter().any(|p| lower.contains(p))
}

/// Find the best repair spec for a user complaint.
pub fn find_spec_for_complaint(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();

    if lower.contains("memory") || lower.contains("remember") || lower.contains("forget") || lower.contains("saving") {
        return Some("001-wire-memory-learn.json");
    }
    if lower.contains("context") || lower.contains("session") || lower.contains("previous") {
        return Some("002-wire-memory-perceive.json");
    }
    if lower.contains("dangerous") || lower.contains("safety") || lower.contains("gate") || lower.contains("risk") {
        return Some("003-wire-execution-gate.json");
    }
    if lower.contains("belief") || lower.contains("preference") || lower.contains("know me") {
        return Some("004-wire-beliefs.json");
    }
    if lower.contains("token") || lower.contains("cost") || lower.contains("expensive") {
        return Some("005-token-optimization.json");
    }
    if lower.contains("web") || lower.contains("browse") || lower.contains("website") {
        return Some("006-web-browsing.json");
    }
    if lower.contains("receipt") || lower.contains("audit") || lower.contains("what did you do") {
        return Some("010-receipt-generation.json");
    }
    if lower.contains("cursor") || lower.contains("ghost") {
        return Some("011-ghost-cursor-overlay.json");
    }
    if lower.contains("screenshot") || lower.contains("screen capture") {
        return Some("012-vision-screen-capture.json");
    }
    if lower.contains("goal") || lower.contains("plan") || lower.contains("track") {
        return Some("015-planning-goals.json");
    }
    if lower.contains("intent") || lower.contains("ambiguous") || lower.contains("understand") {
        return Some("019-veritas-intent-compile.json");
    }
    if lower.contains("dream") || lower.contains("idle") || lower.contains("sleep") {
        return Some("024-dream-state.json");
    }
    if lower.contains("federation") || lower.contains("migrate") || lower.contains("transfer") {
        return Some("025-system-mutation.json");
    }
    if lower.contains("omniscience") || lower.contains("own code") || lower.contains("read yourself") {
        return Some("026-omniscience-loop.json");
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_self_repair_intent() {
        assert!(is_self_repair_intent("fix yourself"));
        assert!(is_self_repair_intent("Hydra, repair your memory"));
        assert!(is_self_repair_intent("run self-repair"));
        assert!(is_self_repair_intent("your memory is broken"));
        assert!(is_self_repair_intent("run diagnostics on yourself"));
        assert!(!is_self_repair_intent("hello"));
        assert!(!is_self_repair_intent("what's the weather"));
        assert!(!is_self_repair_intent("fix this bug in my code"));
    }

    #[test]
    fn test_find_spec_for_complaint() {
        assert_eq!(find_spec_for_complaint("nothing is being saved to memory"), Some("001-wire-memory-learn.json"));
        assert_eq!(find_spec_for_complaint("you don't remember anything"), Some("001-wire-memory-learn.json"));
        assert_eq!(find_spec_for_complaint("the execution gate isn't working"), Some("003-wire-execution-gate.json"));
        assert_eq!(find_spec_for_complaint("you don't know my preferences"), Some("004-wire-beliefs.json"));
        assert_eq!(find_spec_for_complaint("fix your federation"), Some("025-system-mutation.json"));
        assert_eq!(find_spec_for_complaint("hello"), None);
    }

    #[test]
    fn test_evaluate_check_found() {
        let check = AcceptanceCheck {
            name: "test".into(),
            check: "echo hello".into(),
            expect: Some("found".into()),
            expect_minimum: None,
            expect_maximum: None,
        };
        assert!(evaluate_check(&check, "hello world"));
        assert!(!evaluate_check(&check, ""));
        assert!(!evaluate_check(&check, "ERROR: failed"));
    }

    #[test]
    fn test_evaluate_check_not_found() {
        let check = AcceptanceCheck {
            name: "test".into(),
            check: "echo".into(),
            expect: Some("not_found".into()),
            expect_minimum: None,
            expect_maximum: None,
        };
        assert!(evaluate_check(&check, ""));
        assert!(evaluate_check(&check, "0"));
        assert!(!evaluate_check(&check, "some output"));
    }

    #[test]
    fn test_evaluate_check_contains() {
        let check = AcceptanceCheck {
            name: "test".into(),
            check: "echo test".into(),
            expect: Some("Finished".into()),
            expect_minimum: None,
            expect_maximum: None,
        };
        assert!(evaluate_check(&check, "Finished `dev` profile"));
        assert!(!evaluate_check(&check, "error[E0599]"));
    }

    #[test]
    fn test_evaluate_check_minimum() {
        let check = AcceptanceCheck {
            name: "test".into(),
            check: "echo 5".into(),
            expect: None,
            expect_minimum: Some(3),
            expect_maximum: None,
        };
        assert!(evaluate_check(&check, "5"));
        assert!(evaluate_check(&check, "count: 10"));
        assert!(!evaluate_check(&check, "2"));
        assert!(!evaluate_check(&check, "no numbers"));
    }

    #[test]
    fn test_generate_spec() {
        let engine = SelfRepairEngine::new("/tmp/test");
        let spec = engine.generate_spec_from_analysis(
            "Test task",
            "Test description",
            &["file1.rs", "file2.rs"],
            vec![AcceptanceCheck {
                name: "test check".into(),
                check: "echo ok".into(),
                expect: Some("found".into()),
                expect_minimum: None,
                expect_maximum: None,
            }],
            "Fix the thing",
        );
        assert_eq!(spec.task, "Test task");
        assert_eq!(spec.files_to_modify.len(), 2);
        assert_eq!(spec.acceptance_checks.len(), 1);
    }

    #[test]
    fn test_list_specs_empty_dir() {
        let engine = SelfRepairEngine::new("/tmp/nonexistent-hydra-test");
        assert!(engine.list_specs().is_empty());
    }
}
