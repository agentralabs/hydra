//! Sister Self-Improvement Engine — P10.
//!
//! Analyzes a sister codebase, runs baseline tests, identifies limitations,
//! generates patches, applies with checkpoint, tests again, and auto-reverts
//! on regressions.

pub mod analyzer;
pub mod patch_generator;
pub mod verifier;

pub use analyzer::{SisterAnalysis, SisterLanguage};
pub use patch_generator::{ImprovementPatch, PatchRequest};
pub use verifier::{TestResults, VerificationResult};

use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use crate::cognitive::loop_runner::CognitiveUpdate;
use crate::knowledge::KnowledgeAcquirer;

/// Result of an improvement attempt.
#[derive(Debug, Clone)]
pub struct ImprovementReport {
    pub status: ImprovementStatus,
    pub baseline: TestResults,
    pub after: Option<TestResults>,
    pub patch: Option<ImprovementPatch>,
    pub limitation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImprovementStatus {
    /// Patch applied, no regressions, sister improved.
    Success,
    /// Patch caused regressions — reverted.
    Reverted,
    /// Could not identify a limitation.
    NoLimitationFound,
    /// Could not generate a valid patch.
    PatchGenerationFailed,
    /// Analysis or test infrastructure failed.
    Error(String),
}

impl ImprovementReport {
    pub fn success(baseline: TestResults, after: TestResults, patch: ImprovementPatch) -> Self {
        Self {
            status: ImprovementStatus::Success,
            baseline,
            after: Some(after),
            patch: Some(patch),
            limitation: String::new(),
        }
    }

    pub fn reverted(baseline: TestResults, after: TestResults, patch: ImprovementPatch) -> Self {
        Self {
            status: ImprovementStatus::Reverted,
            baseline,
            after: Some(after),
            patch: Some(patch),
            limitation: String::new(),
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            status: ImprovementStatus::Error(msg.to_string()),
            baseline: TestResults::empty(),
            after: None,
            patch: None,
            limitation: String::new(),
        }
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        match &self.status {
            ImprovementStatus::Success => {
                let after = self.after.as_ref().unwrap();
                format!(
                    "Sister improved. {} → {} tests passing. No regressions.",
                    self.baseline.pass_count, after.pass_count
                )
            }
            ImprovementStatus::Reverted => {
                let after = self.after.as_ref().unwrap();
                format!(
                    "Patch caused regressions ({} failures). Reverted to baseline ({} passing).",
                    after.fail_count, self.baseline.pass_count
                )
            }
            ImprovementStatus::NoLimitationFound => {
                "No improvement opportunities identified.".into()
            }
            ImprovementStatus::PatchGenerationFailed => {
                "Could not generate a valid improvement patch.".into()
            }
            ImprovementStatus::Error(msg) => format!("Error: {}", msg),
        }
    }
}

/// Sister improvement engine.
pub struct SisterImprover {
    knowledge: KnowledgeAcquirer,
}

impl SisterImprover {
    pub fn new() -> Self {
        Self { knowledge: KnowledgeAcquirer::new() }
    }

    /// Full improvement pipeline for a sister.
    pub async fn improve(
        &self,
        sister_path: &Path,
        improvement_goal: &str,
        tx: &mpsc::Sender<CognitiveUpdate>,
    ) -> ImprovementReport {
        // STEP 1: Analyze the sister
        send_update(tx, "Analyzing sister codebase...").await;
        let analysis = match analyzer::analyze_sister(sister_path) {
            Ok(a) => a,
            Err(e) => return ImprovementReport::error(&e),
        };
        send_update(tx, &format!(
            "Found {} ({} files, {} test files)",
            analysis.language, analysis.source_files.len(), analysis.test_files.len()
        )).await;

        // STEP 2: Learn from docs
        send_update(tx, "Reading documentation...").await;
        let _docs = self.knowledge.find_docs(sister_path);
        let _prompts = self.knowledge.plan_learning(sister_path);

        // STEP 3: Run baseline tests (30s timeout — may not complete for large projects)
        send_update(tx, "Running baseline tests (30s limit)...").await;
        let baseline = verifier::run_tests(sister_path, &analysis).await;
        let timed_out = baseline.raw_output.contains("timed out");
        if timed_out {
            send_update(tx, "Baseline: tests need compilation (timed out). Analyzing structure instead.").await;
        } else {
            send_update(tx, &format!(
                "Baseline: {} passed, {} failed", baseline.pass_count, baseline.fail_count
            )).await;
        }

        // STEP 4: Identify limitation
        send_update(tx, "Identifying improvement target...").await;
        let limitation = analyzer::identify_limitation(
            &analysis, improvement_goal, &baseline
        );
        if limitation.is_empty() {
            return ImprovementReport {
                status: ImprovementStatus::NoLimitationFound,
                baseline, after: None, patch: None, limitation: String::new(),
            };
        }
        send_update(tx, &format!("Target: {}", limitation)).await;

        // STEP 5: Generate patch
        send_update(tx, "Generating improvement patch...").await;
        let request = PatchRequest {
            sister_path: sister_path.to_path_buf(),
            limitation: limitation.clone(),
            goal: improvement_goal.to_string(),
            analysis: analysis.clone(),
        };
        let patch = match patch_generator::generate_patch(&request) {
            Some(p) => p,
            None => return ImprovementReport {
                status: ImprovementStatus::PatchGenerationFailed,
                baseline, after: None, patch: None, limitation,
            },
        };

        // If patch has no actual changes, report analysis and skip apply
        if patch.changes.is_empty() {
            send_update(tx, "Analysis complete. Patch generation needs LLM (Forge sister or direct).").await;
            return ImprovementReport {
                status: ImprovementStatus::PatchGenerationFailed,
                baseline, after: None, patch: Some(patch), limitation,
            };
        }

        // STEP 6: Create checkpoint and apply patch
        send_update(tx, "Applying patch with checkpoint...").await;
        let checkpoint = create_checkpoint(&patch.target_files);
        if let Err(e) = patch_generator::apply_patch(&patch) {
            let _ = revert_checkpoint(&checkpoint);
            return ImprovementReport::error(&format!("Patch apply failed: {}", e));
        }

        // STEP 7: Run tests after patch (30s timeout)
        send_update(tx, "Running tests after improvement...").await;
        let after = verifier::run_tests(sister_path, &analysis).await;

        // STEP 8: Verify improvement
        let result = verifier::verify(&baseline, &after);
        match result {
            VerificationResult::Improved => {
                send_update(tx, "Improved. No regressions.").await;
                ImprovementReport::success(baseline, after, patch)
            }
            VerificationResult::Regressed => {
                send_update(tx, "Regressions detected. Reverting.").await;
                let _ = revert_checkpoint(&checkpoint);
                ImprovementReport::reverted(baseline, after, patch)
            }
            VerificationResult::Neutral => {
                send_update(tx, "Patch applied. No change in test results.").await;
                ImprovementReport::success(baseline, after, patch)
            }
        }
    }
}

impl Default for SisterImprover {
    fn default() -> Self { Self::new() }
}

/// Extract sister path from user text like "/improve-sister ../agentic-memory add retry".
pub fn extract_sister_path(text: &str) -> Option<PathBuf> {
    let words: Vec<&str> = text.split_whitespace().collect();
    for word in &words {
        let w = word.trim_matches(|c: char| c == '"' || c == '\'');
        // Skip slash commands themselves
        if w.starts_with('/') && !w.contains("..") && !w.starts_with("/tmp")
            && !w.starts_with("/home") && !w.starts_with("/Users") && !w.starts_with("/var") {
            continue;
        }
        if w.contains('/') || w.starts_with('.') || w.starts_with('~') {
            let expanded = if w.starts_with('~') {
                if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                    PathBuf::from(home).join(w.strip_prefix("~/").unwrap_or(w))
                } else {
                    PathBuf::from(w)
                }
            } else {
                PathBuf::from(w)
            };
            if expanded.exists() || expanded.parent().map(|p| p.exists()).unwrap_or(false) {
                return Some(expanded);
            }
        }
    }
    None
}

/// Extract improvement goal from text (everything after the path).
pub fn extract_goal(text: &str) -> String {
    let lower = text.to_lowercase();
    if lower.contains("--auto") {
        return "auto-detect improvements".to_string();
    }
    // Skip command and path, rest is goal
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut goal_start = 0;
    for (i, word) in words.iter().enumerate() {
        if word.contains('/') || word.starts_with('.') || word.starts_with('~') {
            goal_start = i + 1;
            break;
        }
    }
    if goal_start < words.len() {
        words[goal_start..].join(" ")
    } else {
        "auto-detect improvements".to_string()
    }
}

/// Snapshot of files before modification.
#[derive(Debug, Clone)]
struct FileSnapshot {
    path: PathBuf,
    content: String,
    existed: bool,
}

fn create_checkpoint(files: &[PathBuf]) -> Vec<FileSnapshot> {
    files.iter().map(|path| {
        let (content, existed) = if path.exists() {
            (std::fs::read_to_string(path).unwrap_or_default(), true)
        } else {
            (String::new(), false)
        };
        FileSnapshot { path: path.clone(), content, existed }
    }).collect()
}

fn revert_checkpoint(snapshots: &[FileSnapshot]) -> Result<usize, String> {
    let mut reverted = 0;
    for snap in snapshots {
        if snap.existed {
            std::fs::write(&snap.path, &snap.content)
                .map_err(|e| format!("Revert failed {}: {}", snap.path.display(), e))?;
        } else {
            let _ = std::fs::remove_file(&snap.path);
        }
        reverted += 1;
    }
    Ok(reverted)
}

async fn send_update(tx: &mpsc::Sender<CognitiveUpdate>, msg: &str) {
    let _ = tx.try_send(CognitiveUpdate::Phase(msg.to_string()));
}

#[cfg(test)]
#[path = "sister_improve_tests.rs"]
mod tests;
