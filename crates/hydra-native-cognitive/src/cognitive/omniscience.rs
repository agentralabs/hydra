//! Omniscience Loop — Full semantic self-repair via Codebase + Forge + Aegis sisters.
//!
//! The bootstrap `SelfRepairEngine` uses grep-based acceptance checks.
//! This module adds **semantic** self-repair:
//!   1. **Codebase sister** reads Hydra's own source code and builds a semantic graph
//!   2. **Forge sister** generates repair specs from gap analysis
//!   3. **Aegis sister** shadow-validates generated fixes before applying
//!
//! Supports **multi-repo scanning**: Hydra + all 17 sister repos. Each repo gets
//! its own analysis, gap detection, spec generation, and validation.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::cognitive::self_repair::SelfRepairEngine;
// ── Known sister repos (relative to workspace root) ──

/// All known sister project names in the Agentra workspace.
const SISTER_PROJECTS: &[&str] = &[
    "agentic-memory",
    "agentic-identity",
    "agentic-codebase",
    "agentic-vision",
    "agentic-comm",
    "agentic-contract",
    "agentic-time",
    "agentic-planning",
    "agentic-cognition",
    "agentic-reality",
    "agentic-forge",
    "agentic-aegis",
    "agentic-veritas",
    "agentic-evolve",
];

/// A repo target for Omniscience scanning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoTarget {
    /// Human-readable name (e.g. "agentic-hydra", "agentic-memory").
    pub name: String,
    /// Absolute path to the repo root.
    pub path: PathBuf,
    /// Whether this repo exists on disk.
    pub exists: bool,
    /// Primary language (detected from file extensions).
    pub language: String,
}

/// A gap identified by the Omniscience analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniscienceGap {
    /// Which repo this gap belongs to.
    pub repo: String,
    /// Human-readable description of the gap.
    pub description: String,
    /// Files involved (relative to repo root).
    pub files: Vec<String>,
    /// Severity: "critical", "high", "medium", "low".
    pub severity: String,
    /// Category: "missing_implementation", "dead_code", "unconnected_wiring", "missing_test".
    pub category: String,
    /// Suggested fix approach.
    pub suggested_fix: String,
}

/// Per-repo scan result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoScan {
    pub repo: String,
    pub path: String,
    pub gaps: Vec<OmniscienceGap>,
    pub files_analyzed: usize,
    pub health_score: f64,
    pub generated_specs: Vec<String>,
}

/// Result of a full Omniscience scan across all repos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniscienceScan {
    /// Per-repo results.
    pub repo_scans: Vec<RepoScan>,
    /// Aggregated gaps across all repos.
    pub gaps: Vec<OmniscienceGap>,
    pub total_files_analyzed: usize,
    pub code_health_score: f64,
    pub generated_specs: Vec<String>,
}

/// Updates emitted during Omniscience processing.
#[derive(Debug, Clone)]
pub enum OmniscienceUpdate {
    /// Codebase sister is analyzing code.
    CodebaseAnalyzing { phase: String },
    /// Gap identified.
    GapFound(OmniscienceGap),
    /// Forge generated a repair spec.
    SpecGenerated { spec_name: String, task: String },
    /// Aegis validated a fix.
    AegisValidation { spec_name: String, safe: bool, recommendation: String },
    /// Full scan complete.
    ScanComplete(OmniscienceScan),
}

/// The Omniscience engine — orchestrates Codebase + Forge + Aegis for self-repair
/// across Hydra and all sister repos.
pub struct OmniscienceEngine {
    /// Primary repo (agentic-hydra).
    _hydra_root: PathBuf,
    /// Workspace root (parent of all sister repos).
    _workspace_root: PathBuf,
    /// All discovered repo targets.
    pub(crate) targets: Vec<RepoTarget>,
    /// Repair engine (writes specs to hydra's repair-specs/).
    pub(crate) repair_engine: SelfRepairEngine,
}

impl OmniscienceEngine {
    /// Create with just the Hydra repo (backward compatible).
    pub fn new(repo_root: impl Into<PathBuf>) -> Self {
        let root = repo_root.into();
        let workspace = root.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| root.clone());
        let repair_engine = SelfRepairEngine::new(&root);

        let mut targets = vec![RepoTarget {
            name: "agentic-hydra".into(),
            path: root.clone(),
            exists: true,
            language: "rust".into(),
        }];

        // Auto-discover sister repos
        for sister in SISTER_PROJECTS {
            let sister_path = workspace.join(sister);
            let exists = sister_path.exists();
            let language = detect_repo_language(&sister_path);
            targets.push(RepoTarget {
                name: sister.to_string(),
                path: sister_path,
                exists,
                language,
            });
        }

        Self {
            _hydra_root: root,
            _workspace_root: workspace,
            targets,
            repair_engine,
        }
    }

    /// Create with explicit repo paths (for testing or custom configurations).
    pub fn with_targets(hydra_root: impl Into<PathBuf>, extra_repos: &[(&str, &str)]) -> Self {
        let root = hydra_root.into();
        let workspace = root.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| root.clone());
        let repair_engine = SelfRepairEngine::new(&root);

        let mut targets = vec![RepoTarget {
            name: "agentic-hydra".into(),
            path: root.clone(),
            exists: true,
            language: "rust".into(),
        }];

        for (name, path) in extra_repos {
            let p = PathBuf::from(path);
            let exists = p.exists();
            let language = detect_repo_language(&p);
            targets.push(RepoTarget {
                name: name.to_string(),
                path: p,
                exists,
                language,
            });
        }

        Self {
            _hydra_root: root,
            _workspace_root: workspace,
            targets,
            repair_engine,
        }
    }

    /// List all discovered repo targets and their status.
    pub fn list_targets(&self) -> &[RepoTarget] {
        &self.targets
    }

    /// How many sister repos exist on disk.
    pub fn connected_sisters(&self) -> usize {
        self.targets.iter().filter(|t| t.exists && t.name != "agentic-hydra").count()
    }

    // analyze_repo, generate_repo_specs, validate_fix — extracted to omniscience_phases.rs
    // identify_gaps, generate_checks_for_gap — extracted to omniscience_phases.rs
    // apply_fix, run_omniscience_loop — extracted to omniscience_loop.rs
}

// ── Helper functions — extracted to omniscience_scanners.rs ──

pub use super::omniscience_scanners::is_omniscience_intent;
use super::omniscience_scanners::detect_repo_language;
