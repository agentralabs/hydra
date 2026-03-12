//! Self-modification pipeline — spec → gap → patch → verify → apply → revert on failure.
//!
//! Phase 4, Part C: The full pipeline for Hydra to modify its own code from a spec.
//! Safety: checkpoint always created before patching, auto-revert on any failure,
//! human approval required for critical files.

pub use crate::self_modify_pipeline::SelfModificationPipeline;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Result of a self-modification pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModResult {
    /// All gaps filled, patches applied, tests pass.
    Success {
        gaps_filled: usize,
        patches_applied: usize,
        tests_passing: usize,
    },
    /// The spec is already fully implemented.
    AlreadyImplemented,
    /// Shadow validation blocked a patch.
    ShadowFailed {
        reason: String,
        patch_summary: String,
    },
    /// Compilation failed after applying patches — auto-reverted.
    CompileFailed {
        error: String,
        reverted: bool,
    },
    /// Tests failed after applying patches — auto-reverted.
    TestsFailed {
        failures: Vec<String>,
        reverted: bool,
    },
    /// Pipeline error (IO, parse, etc.)
    PipelineError {
        message: String,
    },
}

impl ModResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Human-readable summary for the UI.
    pub fn summary(&self) -> String {
        match self {
            Self::Success { gaps_filled, patches_applied, tests_passing } => {
                format!(
                    "Done. Found {} gaps, applied {} patches, {} tests passing.",
                    gaps_filled, patches_applied, tests_passing
                )
            }
            Self::AlreadyImplemented => {
                "This capability already exists in the codebase.".to_string()
            }
            Self::ShadowFailed { reason, .. } => {
                format!("Shadow validation blocked this patch: {}", reason)
            }
            Self::CompileFailed { error, reverted } => {
                format!(
                    "Patch didn't compile. {}. Error: {}",
                    if *reverted { "Reverted" } else { "NOT reverted" },
                    error
                )
            }
            Self::TestsFailed { failures, reverted } => {
                format!(
                    "{} tests failed. {}. Failures: {}",
                    failures.len(),
                    if *reverted { "Reverted" } else { "NOT reverted" },
                    failures.join(", ")
                )
            }
            Self::PipelineError { message } => {
                format!("Pipeline error: {}", message)
            }
        }
    }
}

/// A gap between what a spec requires and what currently exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecGap {
    pub description: String,
    pub target_file: String,
    pub gap_type: GapType,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapType {
    MissingFunction,
    MissingModule,
    MissingTest,
    MissingIntegration,
    IncompleteImplementation,
}

/// A generated patch (diff) to fill a gap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub target_file: String,
    pub gap: SpecGap,
    pub diff_content: String,
    pub description: String,
    /// Whether this patch touches a critical safety file.
    pub touches_critical: bool,
}

impl Patch {
    /// Check if this patch touches critical Hydra safety infrastructure.
    pub fn requires_extra_approval(&self) -> bool {
        let critical_patterns = [
            "execution_gate",
            "receipt_ledger",
            "kill_switch",
            "boundary_enforcer",
            "constitutional",
        ];
        critical_patterns
            .iter()
            .any(|p| self.target_file.contains(p) || self.diff_content.contains(p))
    }
}

/// A checkpoint that can be used to revert changes.
#[derive(Debug, Clone)]
pub struct FileCheckpoint {
    pub id: String,
    pub files: Vec<FileSnapshot>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub content: String,
    pub existed: bool,
}

impl FileCheckpoint {
    /// Create a checkpoint by snapshotting files that will be modified.
    pub fn capture(files_to_modify: &[PathBuf]) -> Self {
        let snapshots: Vec<FileSnapshot> = files_to_modify
            .iter()
            .map(|path| {
                let (content, existed) = if path.exists() {
                    (
                        std::fs::read_to_string(path).unwrap_or_default(),
                        true,
                    )
                } else {
                    (String::new(), false)
                };
                FileSnapshot {
                    path: path.clone(),
                    content,
                    existed,
                }
            })
            .collect();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            files: snapshots,
            created_at: chrono::Utc::now(),
        }
    }

    /// Revert all files to their checkpoint state.
    pub fn revert(&self) -> Result<usize, String> {
        let mut reverted = 0;
        for snapshot in &self.files {
            if snapshot.existed {
                std::fs::write(&snapshot.path, &snapshot.content)
                    .map_err(|e| format!("Failed to revert {}: {}", snapshot.path.display(), e))?;
            } else {
                // File didn't exist before — remove it
                let _ = std::fs::remove_file(&snapshot.path);
            }
            reverted += 1;
        }
        Ok(reverted)
    }
}

/// Extract function name from a signature line.
pub(crate) fn extract_fn_name(sig: &str) -> String {
    let trimmed = sig.trim();
    let after_fn = if let Some(rest) = trimmed.strip_prefix("pub async fn ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("pub fn ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("async fn ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("fn ") {
        rest
    } else {
        return String::new();
    };

    after_fn
        .split(|c: char| c == '(' || c == '<' || c.is_whitespace())
        .next()
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
#[path = "self_modify_tests.rs"]
mod tests;
