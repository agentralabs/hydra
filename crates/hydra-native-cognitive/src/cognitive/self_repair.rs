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

use serde::{Deserialize, Serialize};

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
    pub(crate) repo_root: PathBuf,
    pub(crate) specs_dir: PathBuf,
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

    /// Validate a check command against an allowlist of safe prefixes.
    fn is_safe_check_command(cmd: &str) -> bool {
        let safe_prefixes = ["cargo ", "wc ", "grep ", "test ", "cat ", "ls ", "head ", "tail ", "echo ", "diff "];
        let trimmed = cmd.trim();
        safe_prefixes.iter().any(|p| trimmed.starts_with(p))
            || trimmed.starts_with('[') // shell test bracket
            || trimmed.contains("| wc") || trimmed.contains("| grep")
    }

    /// Run a single acceptance check. Returns (passed, output).
    pub async fn run_check(&self, check: &AcceptanceCheck) -> CheckResult {
        // Validate command against safe allowlist
        if !Self::is_safe_check_command(&check.check) {
            eprintln!("[hydra:self-repair] BLOCKED unsafe check command: {}", check.check);
            return CheckResult {
                name: check.name.clone(),
                passed: false,
                output: format!("BLOCKED: command not in safe allowlist: {}", check.check),
            };
        }

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
pub fn evaluate_check(check: &AcceptanceCheck, output: &str) -> bool {
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
#[path = "self_repair_tests.rs"]
mod tests;
