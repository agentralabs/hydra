//! Omniscience Loop — Full semantic self-repair via Codebase + Forge + Aegis sisters.
//!
//! The bootstrap `SelfRepairEngine` uses grep-based acceptance checks.
//! This module adds **semantic** self-repair:
//!   1. **Codebase sister** reads Hydra's own source code and builds a semantic graph
//!   2. **Forge sister** generates repair specs from gap analysis
//!   3. **Aegis sister** shadow-validates generated fixes before applying
//!
//! Supports **multi-repo scanning**: Hydra + all 14 sister repos. Each repo gets
//! its own analysis, gap detection, spec generation, and validation.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::cognitive::self_repair::{
    AcceptanceCheck, RepairSpec, SelfRepairEngine,
};
use crate::sisters::Sisters;

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
    targets: Vec<RepoTarget>,
    /// Repair engine (writes specs to hydra's repair-specs/).
    repair_engine: SelfRepairEngine,
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

    /// Phase 1: Use the Codebase sister to analyze a specific repo.
    pub async fn analyze_repo(&self, sisters: &Sisters, target: &RepoTarget) -> Option<String> {
        let codebase = sisters.codebase.as_ref()?;

        let repo_path = target.path.display().to_string();

        // Actual Codebase sister tool names (verified from agentic-codebase MCP registry)
        let core_analysis = codebase.call_tool("search_semantic", serde_json::json!({
            "query": format!("analyze the full implementation in {}", repo_path),
        })).await.ok();

        let stub_analysis = codebase.call_tool("impact_analyze", serde_json::json!({
            "query": format!("which functions in {} are stubs or unimplemented", target.name),
        })).await.ok();

        let concept_analysis = codebase.call_tool("concept_map", serde_json::json!({
            "query": format!("{} architecture: modules, connections, and missing wiring", target.name),
        })).await.ok();

        let mut result = String::new();
        if let Some(v) = core_analysis {
            if let Some(text) = v.as_str() {
                result.push_str(&format!("## {} Core Analysis\n{}\n", target.name, text));
            }
        }
        if let Some(v) = stub_analysis {
            if let Some(text) = v.as_str() {
                result.push_str(&format!("## {} Stub Analysis\n{}\n", target.name, text));
            }
        }
        if let Some(v) = concept_analysis {
            if let Some(text) = v.as_str() {
                result.push_str(&format!("## {} Concept Map\n{}\n", target.name, text));
            }
        }

        if result.is_empty() { None } else { Some(result) }
    }

    /// Phase 2: Generate repair specs for gaps in a specific repo.
    pub async fn generate_repo_specs(
        &self,
        sisters: &Sisters,
        target: &RepoTarget,
        gaps: &[OmniscienceGap],
    ) -> Vec<(String, RepairSpec)> {
        let forge = match sisters.forge.as_ref() {
            Some(f) => f,
            None => return vec![],
        };

        let mut specs = Vec::new();
        let repo_short = target.name.replace("agentic-", "");

        for (i, gap) in gaps.iter().enumerate() {
            let spec_name = format!("omni-{}-{:03}-{}.json",
                repo_short,
                i + 1,
                gap.category.replace('_', "-")
            );

            // Actual Forge tool name: forge_blueprint_create
            let blueprint = forge.call_tool("forge_blueprint_create", serde_json::json!({
                "intent": format!(
                    "Fix gap in {} ({}): {}\n\nFiles: {:?}\nSeverity: {}\nSuggested approach: {}",
                    target.name, target.language,
                    gap.description, gap.files, gap.severity, gap.suggested_fix
                ),
            })).await.ok();

            let instructions = if let Some(ref bp) = blueprint {
                bp.as_str().unwrap_or(&gap.suggested_fix).to_string()
            } else {
                gap.suggested_fix.clone()
            };

            let checks = self.generate_checks_for_gap(target, gap);

            let spec = self.repair_engine.generate_spec_from_analysis(
                &format!("[{}] {}", target.name, gap.description),
                &gap.description,
                &gap.files.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                checks,
                &format!("REPO: {}\nPATH: {}\n\n{}", target.name, target.path.display(), instructions),
            );

            specs.push((spec_name, spec));
        }

        specs
    }

    /// Phase 3: Aegis shadow-validate a fix for a specific repo.
    pub async fn validate_fix(
        &self,
        sisters: &Sisters,
        target: &RepoTarget,
        spec: &RepairSpec,
    ) -> (bool, String) {
        let aegis = match sisters.aegis.as_ref() {
            Some(a) => a,
            None => return (true, "Aegis not available — skipping validation".to_string()),
        };

        let check_cmd = match target.language.as_str() {
            "rust" => format!("cd {} && cargo check 2>&1 | tail -1", target.path.display()),
            "typescript" | "javascript" => format!("cd {} && npx tsc --noEmit 2>&1 | tail -5", target.path.display()),
            "python" => format!("cd {} && python -m py_compile *.py 2>&1", target.path.display()),
            _ => format!("cd {} && ls -la", target.path.display()),
        };

        // Actual Aegis tool name: aegis_shadow_execute
        let result = aegis.call_tool("aegis_shadow_execute", serde_json::json!({
            "command": format!("{} after applying: {}", check_cmd, &spec.instructions_for_claude_code[..spec.instructions_for_claude_code.len().min(200)]),
            "dry_run": true,
            "scope": target.name,
        })).await.ok();

        if let Some(v) = result {
            let safe = v.get("safe").and_then(|s| s.as_bool()).unwrap_or(true);
            let rec = v.as_str()
                .or_else(|| v.get("recommendation").and_then(|r| r.as_str()))
                .unwrap_or("No recommendation")
                .to_string();
            (safe, rec)
        } else {
            (true, "Aegis returned no result — proceeding with caution".to_string())
        }
    }

    /// Phase 5: Apply a fix using the Codebase→Forge→Aegis pipeline.
    ///
    /// This is the CORRECT way to fix code — never blind sed.
    /// 1. Codebase reads and understands the file
    /// 2. Forge generates the replacement code
    /// 3. Aegis validates the change is safe
    /// 4. Returns the fix instructions for the LLM to apply with full context
    pub async fn apply_fix(
        &self,
        sisters: &Sisters,
        target: &RepoTarget,
        gap: &OmniscienceGap,
    ) -> Option<String> {
        let codebase = sisters.codebase.as_ref()?;
        let forge = sisters.forge.as_ref();

        // STEP 1: UNDERSTAND — Read the actual file and understand context
        let file_path = gap.files.first()?;
        let full_path = if file_path.starts_with('/') {
            file_path.clone()
        } else {
            format!("{}/{}", target.path.display(), file_path)
        };

        // search_semantic: full semantic search across codebase
        let code_context = codebase.call_tool("search_semantic", serde_json::json!({
            "query": format!("Read and analyze {}: what does this file do, what are its dependencies, what calls into it", full_path),
        })).await.ok();

        // impact_analyze: what would be affected by this change
        let impact = codebase.call_tool("impact_analyze", serde_json::json!({
            "query": format!("What would be affected if we modify {} at the gap: {}", file_path, gap.description),
        })).await.ok();

        // STEP 2: PLAN — Generate a blueprint for the fix
        let blueprint = if let Some(forge) = forge {
            // forge_blueprint_create: generates a structured fix plan
            forge.call_tool("forge_blueprint_create", serde_json::json!({
                "intent": format!(
                    "Fix this gap in {} ({}):\n\nGap: {}\nFile: {}\nSeverity: {}\n\nCode context:\n{}\n\nImpact analysis:\n{}",
                    target.name, target.language,
                    gap.description, file_path, gap.severity,
                    code_context.as_ref().and_then(|v| v.as_str()).unwrap_or("unavailable"),
                    impact.as_ref().and_then(|v| v.as_str()).unwrap_or("unavailable"),
                ),
            })).await.ok()
        } else {
            None
        };

        // STEP 3: VALIDATE — Shadow-execute before applying
        let aegis_ok = if let Some(aegis) = sisters.aegis.as_ref() {
            // aegis_shadow_execute: dry-run validation before applying
            let validation = aegis.call_tool("aegis_shadow_execute", serde_json::json!({
                "command": format!("Proposed fix for {} in {}: {}", file_path, target.name, gap.suggested_fix),
                "dry_run": true,
                "scope": target.name,
            })).await.ok();
            validation
                .and_then(|v| v.get("safe").and_then(|s| s.as_bool()))
                .unwrap_or(true)
        } else {
            true
        };

        if !aegis_ok {
            return Some(format!(
                "Aegis blocked fix for {}: {} — unsafe change detected",
                file_path, gap.description
            ));
        }

        // STEP 4: Build fix instructions with full context
        let mut instructions = String::new();
        instructions.push_str(&format!("## Fix for {} in {}\n\n", file_path, target.name));
        instructions.push_str(&format!("**Gap:** {}\n", gap.description));
        instructions.push_str(&format!("**Severity:** {}\n\n", gap.severity));

        if let Some(ref bp) = blueprint {
            if let Some(text) = bp.as_str() {
                instructions.push_str(&format!("### Blueprint\n{}\n\n", text));
            }
        }

        if let Some(ref ctx) = code_context {
            if let Some(text) = ctx.as_str() {
                instructions.push_str(&format!("### Code Context\n{}\n\n", text));
            }
        }

        if let Some(ref imp) = impact {
            if let Some(text) = imp.as_str() {
                instructions.push_str(&format!("### Impact Analysis\n{}\n\n", text));
            }
        }

        // STEP 5: Verify command
        let verify_cmd = match target.language.as_str() {
            "rust" => format!("cd {} && cargo check && cargo test", target.path.display()),
            "typescript" | "javascript" => format!("cd {} && npx tsc --noEmit && npm test", target.path.display()),
            "python" => format!("cd {} && python -m pytest", target.path.display()),
            _ => format!("echo 'Manual verification required for {}'", target.language),
        };
        instructions.push_str(&format!("### Verify After Applying\n```\n{}\n```\n", verify_cmd));

        Some(instructions)
    }

    /// Run the full Omniscience Loop across ALL repos.
    pub async fn run_omniscience_loop(
        &self,
        sisters: &Sisters,
        tx: Option<&mpsc::UnboundedSender<OmniscienceUpdate>>,
    ) -> OmniscienceScan {
        let mut all_gaps = Vec::new();
        let mut all_specs = Vec::new();
        let mut total_files = 0;
        let mut repo_scans = Vec::new();

        let existing_targets: Vec<&RepoTarget> = self.targets.iter()
            .filter(|t| t.exists)
            .collect();

        for target in &existing_targets {
            if let Some(tx) = tx {
                let _ = tx.send(OmniscienceUpdate::CodebaseAnalyzing {
                    phase: format!("Scanning {} ({})...", target.name, target.language),
                });
            }

            // Phase 1: Analyze this repo
            let analysis = self.analyze_repo(sisters, target).await;

            // Phase 2: Identify gaps (semantic + filesystem scan)
            let mut gaps = self.identify_gaps(target, &analysis.unwrap_or_default());
            let files_count = count_source_files_in(&target.path, &target.language);
            total_files += files_count;

            for gap in &gaps {
                if let Some(tx) = tx {
                    let _ = tx.send(OmniscienceUpdate::GapFound(gap.clone()));
                }
            }

            // Phase 3: Generate specs via Forge
            let generated = self.generate_repo_specs(sisters, target, &gaps).await;
            let mut spec_names = Vec::new();

            for (name, spec) in &generated {
                if let Some(tx) = tx {
                    let _ = tx.send(OmniscienceUpdate::SpecGenerated {
                        spec_name: name.clone(),
                        task: spec.task.clone(),
                    });
                }

                // Phase 4: Validate via Aegis
                let (safe, recommendation) = self.validate_fix(sisters, target, spec).await;
                if let Some(tx) = tx {
                    let _ = tx.send(OmniscienceUpdate::AegisValidation {
                        spec_name: name.clone(),
                        safe,
                        recommendation: recommendation.clone(),
                    });
                }

                if safe {
                    if let Ok(_) = self.repair_engine.save_spec(spec, name) {
                        spec_names.push(name.clone());
                        all_specs.push(name.clone());
                    }
                }
            }

            let health = calculate_health_score(&gaps, files_count);
            repo_scans.push(RepoScan {
                repo: target.name.clone(),
                path: target.path.display().to_string(),
                gaps: gaps.clone(),
                files_analyzed: files_count,
                health_score: health,
                generated_specs: spec_names,
            });

            all_gaps.append(&mut gaps);
        }

        let overall_health = if total_files > 0 {
            calculate_health_score(&all_gaps, total_files)
        } else {
            1.0
        };

        let scan = OmniscienceScan {
            repo_scans,
            total_files_analyzed: total_files,
            code_health_score: overall_health,
            gaps: all_gaps,
            generated_specs: all_specs,
        };

        if let Some(tx) = tx {
            let _ = tx.send(OmniscienceUpdate::ScanComplete(scan.clone()));
        }

        scan
    }

    /// Identify gaps in a specific repo from analysis text + filesystem scan.
    fn identify_gaps(&self, target: &RepoTarget, analysis: &str) -> Vec<OmniscienceGap> {
        let mut gaps = Vec::new();

        // Pattern-based gap detection from Codebase sister analysis
        let gap_patterns = [
            ("todo!", "missing_implementation", "critical"),
            ("unimplemented!", "missing_implementation", "critical"),
            ("stub", "missing_implementation", "high"),
            ("// TODO", "missing_implementation", "medium"),
            ("not wired", "unconnected_wiring", "high"),
            ("not connected", "unconnected_wiring", "high"),
            ("dead code", "dead_code", "low"),
            ("unused", "dead_code", "low"),
            ("no test", "missing_test", "medium"),
            ("untested", "missing_test", "medium"),
        ];

        for (pattern, category, severity) in &gap_patterns {
            if analysis.to_lowercase().contains(&pattern.to_lowercase()) {
                gaps.push(OmniscienceGap {
                    repo: target.name.clone(),
                    description: format!("[{}] Found '{}' in analysis", target.name, pattern),
                    files: vec![],
                    severity: severity.to_string(),
                    category: category.to_string(),
                    suggested_fix: format!("Implement or remove the '{}' marker in {}", pattern, target.name),
                });
            }
        }

        // Direct filesystem scan based on language
        match target.language.as_str() {
            "rust" => scan_rust_stubs(&target.path, &target.name, &mut gaps),
            "typescript" | "javascript" => scan_ts_stubs(&target.path, &target.name, &mut gaps),
            _ => {} // Python and others: rely on Codebase sister analysis
        }

        gaps.dedup_by(|a, b| a.description == b.description);
        gaps
    }

    /// Generate acceptance checks for a gap in a specific repo.
    fn generate_checks_for_gap(&self, target: &RepoTarget, gap: &OmniscienceGap) -> Vec<AcceptanceCheck> {
        let mut checks = Vec::new();
        let repo_path = target.path.display().to_string();

        match gap.category.as_str() {
            "missing_implementation" => {
                for file in &gap.files {
                    let full_path = if file.starts_with('/') {
                        file.clone()
                    } else {
                        format!("{}/{}", repo_path, file)
                    };
                    let stub_pattern = match target.language.as_str() {
                        "rust" => "todo!\\|unimplemented!",
                        "typescript" | "javascript" => "throw new Error.*not implemented\\|TODO",
                        "python" => "raise NotImplementedError\\|pass  # TODO",
                        _ => "TODO\\|FIXME\\|HACK",
                    };
                    checks.push(AcceptanceCheck {
                        name: format!("no-stubs-in-{}", file.replace('/', "-")),
                        check: format!("grep -c '{}' {} || echo 0", stub_pattern, full_path),
                        expect: Some("not_found".into()),
                        expect_minimum: None,
                        expect_maximum: None,
                    });
                }
                // Compilation check appropriate to the language
                let compile_check = match target.language.as_str() {
                    "rust" => {
                        // Use the Cargo package name if it's a workspace member
                        let _pkg = target.name.replace("agentic-", "");
                        format!("cd {} && cargo check 2>&1 | tail -1", repo_path)
                    }
                    "typescript" => format!("cd {} && npx tsc --noEmit 2>&1 | tail -1", repo_path),
                    "javascript" => format!("cd {} && node -c src/index.js 2>&1", repo_path),
                    "python" => format!("cd {} && python -m py_compile $(find . -name '*.py' -not -path './venv/*' | head -5) 2>&1", repo_path),
                    _ => format!("ls {}", repo_path),
                };
                checks.push(AcceptanceCheck {
                    name: format!("{}-compiles", target.name),
                    check: compile_check,
                    expect: Some("Finished".into()),
                    expect_minimum: None,
                    expect_maximum: None,
                });
            }
            "unconnected_wiring" => {
                let test_check = match target.language.as_str() {
                    "rust" => format!("cd {} && cargo test 2>&1 | tail -1", repo_path),
                    "typescript" | "javascript" => format!("cd {} && npm test 2>&1 | tail -3", repo_path),
                    "python" => format!("cd {} && python -m pytest 2>&1 | tail -3", repo_path),
                    _ => format!("echo ok"),
                };
                checks.push(AcceptanceCheck {
                    name: format!("{}-tests-pass", target.name),
                    check: test_check,
                    expect: Some("passed".into()),
                    expect_minimum: None,
                    expect_maximum: None,
                });
            }
            "missing_test" => {
                let test_pattern = match target.language.as_str() {
                    "rust" => "#\\[test\\]",
                    "typescript" | "javascript" => "describe\\|it\\(\\|test\\(",
                    "python" => "def test_\\|class Test",
                    _ => "test",
                };
                let search_dir = gap.files.first()
                    .map(|f| format!("{}/{}", repo_path, f))
                    .unwrap_or_else(|| format!("{}/src", repo_path));
                checks.push(AcceptanceCheck {
                    name: format!("{}-test-exists", target.name),
                    check: format!("grep -r '{}' {} | wc -l", test_pattern, search_dir),
                    expect: None,
                    expect_minimum: Some(1),
                    expect_maximum: None,
                });
            }
            _ => {
                let compile_check = match target.language.as_str() {
                    "rust" => format!("cd {} && cargo check 2>&1 | tail -1", repo_path),
                    "typescript" => format!("cd {} && npx tsc --noEmit 2>&1 | tail -1", repo_path),
                    _ => format!("echo ok"),
                };
                checks.push(AcceptanceCheck {
                    name: format!("{}-compiles", target.name),
                    check: compile_check,
                    expect: Some("Finished".into()),
                    expect_minimum: None,
                    expect_maximum: None,
                });
            }
        }

        checks
    }
}

// ── Helper functions ──

/// Detect the primary language of a repo from its files.
fn detect_repo_language(path: &PathBuf) -> String {
    if path.join("Cargo.toml").exists() {
        "rust".into()
    } else if path.join("tsconfig.json").exists() {
        "typescript".into()
    } else if path.join("package.json").exists() {
        "javascript".into()
    } else if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
        "python".into()
    } else {
        "unknown".into()
    }
}

/// Count source files in a repo based on language.
fn count_source_files_in(root: &PathBuf, language: &str) -> usize {
    let extensions: &[&str] = match language {
        "rust" => &["rs"],
        "typescript" => &["ts", "tsx"],
        "javascript" => &["js", "jsx", "mjs"],
        "python" => &["py"],
        _ => &["rs", "ts", "js", "py"],
    };

    fn count_files(dir: &std::path::Path, exts: &[&str]) -> usize {
        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name != "target" && name != "node_modules" && name != ".git"
                        && name != "dist" && name != "build" && name != "__pycache__"
                        && !name.starts_with('.')
                    {
                        count += count_files(&path, exts);
                    }
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if exts.contains(&ext) {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    count_files(root, extensions)
}

/// Scan a Rust repo for todo!()/unimplemented!() stubs.
///
/// Excludes false positives:
/// - `todo!()` inside string literals (template generators, push_str, format!)
/// - `todo!()` inside assert!() macros (test assertions)
/// - Files in tests/ and benches/ directories (test fixtures)
/// - Lines that are comments
fn scan_rust_stubs(root: &PathBuf, repo_name: &str, gaps: &mut Vec<OmniscienceGap>) {
    fn is_false_positive(line: &str, rel_path: &str) -> bool {
        let trimmed = line.trim();

        // Skip test fixtures and benchmarks
        if rel_path.starts_with("tests/") || rel_path.starts_with("benches/") {
            return true;
        }

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
            return true;
        }

        // Skip string literals: todo!() inside quotes means it's template output
        // Matches: "...todo!()...", push_str("...todo!()..."), format!("...todo!()...")
        if trimmed.contains("\"") {
            // Check if todo!() appears inside a quoted string
            let chars: Vec<char> = trimmed.chars().collect();
            let todo_pattern = "todo!()";
            let unimpl_pattern = "unimplemented!()";

            // Find all positions of todo!()/unimplemented!() and check if they're inside strings
            for pattern in &[todo_pattern, unimpl_pattern] {
                if let Some(pos) = trimmed.find(pattern) {
                    // Count unescaped quotes before this position
                    let mut quotes = 0;
                    let mut prev_was_escape = false;
                    for (i, ch) in chars.iter().enumerate() {
                        if i >= pos { break; }
                        if prev_was_escape {
                            prev_was_escape = false;
                            continue;
                        }
                        if *ch == '\\' {
                            prev_was_escape = true;
                            continue;
                        }
                        if *ch == '"' {
                            quotes += 1;
                        }
                    }
                    // Odd number of quotes before = inside a string literal
                    if quotes % 2 == 1 {
                        return true;
                    }
                }
            }
        }

        // Skip assert!() macros containing todo!()
        if trimmed.contains("assert") && (trimmed.contains("todo!()") || trimmed.contains("unimplemented!()")) {
            return true;
        }

        false
    }

    fn scan_dir(dir: &std::path::Path, root: &std::path::Path, repo_name: &str, gaps: &mut Vec<OmniscienceGap>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name != "target" && !name.starts_with('.') {
                        scan_dir(&path, root, repo_name, gaps);
                    }
                } else if path.extension().map_or(false, |e| e == "rs") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let rel_path = path.strip_prefix(root)
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| path.display().to_string());

                        for (line_num, line) in content.lines().enumerate() {
                            if (line.contains("todo!()") || line.contains("unimplemented!()"))
                                && !is_false_positive(line, &rel_path)
                            {
                                gaps.push(OmniscienceGap {
                                    repo: repo_name.to_string(),
                                    description: format!("[{}] {}:{} — {}", repo_name, rel_path, line_num + 1, line.trim()),
                                    files: vec![rel_path.clone()],
                                    severity: "critical".into(),
                                    category: "missing_implementation".into(),
                                    suggested_fix: format!("Implement the stub at {}:{}", rel_path, line_num + 1),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    scan_dir(root, root, repo_name, gaps);
}

/// Scan a TypeScript/JavaScript repo for unimplemented stubs.
///
/// Excludes false positives:
/// - Files in __tests__/, tests/, *.test.ts, *.spec.ts (test fixtures)
/// - Lines inside string literals (template output)
/// - Comments
fn scan_ts_stubs(root: &PathBuf, repo_name: &str, gaps: &mut Vec<OmniscienceGap>) {
    fn is_test_file(rel_path: &str) -> bool {
        rel_path.starts_with("tests/")
            || rel_path.starts_with("__tests__/")
            || rel_path.contains(".test.")
            || rel_path.contains(".spec.")
            || rel_path.starts_with("test/")
    }

    fn scan_dir(dir: &std::path::Path, root: &std::path::Path, repo_name: &str, gaps: &mut Vec<OmniscienceGap>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name != "node_modules" && name != "dist" && name != ".next"
                        && !name.starts_with('.')
                    {
                        scan_dir(&path, root, repo_name, gaps);
                    }
                } else if path.extension().map_or(false, |e| e == "ts" || e == "tsx" || e == "js") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let rel_path = path.strip_prefix(root)
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| path.display().to_string());

                        // Skip test files entirely
                        if is_test_file(&rel_path) {
                            continue;
                        }

                        for (line_num, line) in content.lines().enumerate() {
                            let trimmed = line.trim();
                            // Skip comments
                            if trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("/*") {
                                continue;
                            }
                            if trimmed.contains("throw new Error") && trimmed.to_lowercase().contains("not implemented") {
                                gaps.push(OmniscienceGap {
                                    repo: repo_name.to_string(),
                                    description: format!("[{}] {}:{} — {}", repo_name, rel_path, line_num + 1, trimmed),
                                    files: vec![rel_path.clone()],
                                    severity: "critical".into(),
                                    category: "missing_implementation".into(),
                                    suggested_fix: format!("Implement the stub at {}:{}", rel_path, line_num + 1),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    scan_dir(root, root, repo_name, gaps);
}

/// Calculate health score from gaps and file count.
fn calculate_health_score(gaps: &[OmniscienceGap], total_files: usize) -> f64 {
    let total = total_files.max(1) as f64;
    let critical = gaps.iter().filter(|g| g.severity == "critical").count() as f64;
    let high = gaps.iter().filter(|g| g.severity == "high").count() as f64;
    let medium = gaps.iter().filter(|g| g.severity == "medium").count() as f64;
    let penalty = (critical * 10.0 + high * 5.0 + medium * 2.0) / total;
    (1.0 - penalty).max(0.0).min(1.0)
}

/// Detect if user input is an omniscience intent.
pub fn is_omniscience_intent(text: &str) -> bool {
    let lower = text.to_lowercase();
    let patterns = [
        "omniscience", "read your own code", "read yourself",
        "analyze your code", "scan yourself", "full self-analysis",
        "understand your own", "code health", "gap analysis",
        "semantic repair", "deep self-repair", "full scan",
        "scan all sisters", "repair sisters", "fix sisters",
        "scan all repos", "check all systems",
    ];
    patterns.iter().any(|p| lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_omniscience_intent() {
        assert!(is_omniscience_intent("run omniscience loop"));
        assert!(is_omniscience_intent("read your own code"));
        assert!(is_omniscience_intent("do a full scan"));
        assert!(is_omniscience_intent("analyze your code for gaps"));
        assert!(is_omniscience_intent("scan all sisters"));
        assert!(is_omniscience_intent("repair sisters"));
        assert!(!is_omniscience_intent("hello"));
        assert!(!is_omniscience_intent("fix this bug"));
    }

    #[test]
    fn test_detect_repo_language() {
        // Non-existent paths should return "unknown"
        let p = PathBuf::from("/tmp/nonexistent-test-repo");
        assert_eq!(detect_repo_language(&p), "unknown");
    }

    #[test]
    fn test_identify_gaps_from_text() {
        let engine = OmniscienceEngine::new("/tmp/nonexistent-test");
        let target = RepoTarget {
            name: "test-repo".into(),
            path: "/tmp/nonexistent-test".into(),
            exists: false,
            language: "rust".into(),
        };
        let analysis = "Found a stub implementation in the federation module. \
                        There is also dead code in the old parser.";
        let gaps = engine.identify_gaps(&target, analysis);
        assert!(gaps.iter().any(|g| g.category == "missing_implementation"));
        assert!(gaps.iter().any(|g| g.category == "dead_code"));
        assert!(gaps.iter().all(|g| g.repo == "test-repo"));
    }

    #[test]
    fn test_health_score() {
        assert_eq!(calculate_health_score(&[], 100), 1.0);

        let gaps = vec![
            OmniscienceGap {
                repo: "test".into(),
                description: "test".into(),
                files: vec![],
                severity: "critical".into(),
                category: "missing_implementation".into(),
                suggested_fix: "fix it".into(),
            },
        ];
        let score = calculate_health_score(&gaps, 100);
        assert!(score < 1.0);
        assert!(score > 0.0);
    }

    #[test]
    fn test_multi_repo_targets() {
        let engine = OmniscienceEngine::new("/tmp/nonexistent-hydra");
        // Should have hydra + 14 sisters = 15 targets
        assert_eq!(engine.targets.len(), 15);
        assert_eq!(engine.targets[0].name, "agentic-hydra");
        assert!(engine.targets.iter().any(|t| t.name == "agentic-memory"));
        assert!(engine.targets.iter().any(|t| t.name == "agentic-aegis"));
        assert!(engine.targets.iter().any(|t| t.name == "agentic-forge"));
    }

    #[test]
    fn test_with_explicit_targets() {
        let engine = OmniscienceEngine::with_targets("/tmp/hydra", &[
            ("custom-sister", "/tmp/custom"),
        ]);
        assert_eq!(engine.targets.len(), 2);
        assert_eq!(engine.targets[0].name, "agentic-hydra");
        assert_eq!(engine.targets[1].name, "custom-sister");
    }

    #[test]
    fn test_generate_checks_rust() {
        let engine = OmniscienceEngine::new("/tmp/test");
        let target = RepoTarget {
            name: "agentic-memory".into(),
            path: "/tmp/agentic-memory".into(),
            exists: true,
            language: "rust".into(),
        };
        let gap = OmniscienceGap {
            repo: "agentic-memory".into(),
            description: "stub in lib.rs".into(),
            files: vec!["src/lib.rs".into()],
            severity: "critical".into(),
            category: "missing_implementation".into(),
            suggested_fix: "implement it".into(),
        };
        let checks = engine.generate_checks_for_gap(&target, &gap);
        assert!(checks.len() >= 2);
        assert!(checks.iter().any(|c| c.name.contains("no-stubs")));
        assert!(checks.iter().any(|c| c.name.contains("agentic-memory-compiles")));
    }

    #[test]
    fn test_generate_checks_typescript() {
        let engine = OmniscienceEngine::new("/tmp/test");
        let target = RepoTarget {
            name: "agentic-vision".into(),
            path: "/tmp/agentic-vision".into(),
            exists: true,
            language: "typescript".into(),
        };
        let gap = OmniscienceGap {
            repo: "agentic-vision".into(),
            description: "missing test".into(),
            files: vec!["src/capture.ts".into()],
            severity: "medium".into(),
            category: "missing_test".into(),
            suggested_fix: "add tests".into(),
        };
        let checks = engine.generate_checks_for_gap(&target, &gap);
        assert!(checks.iter().any(|c| c.check.contains("describe")));
    }

    #[test]
    fn test_count_source_files() {
        // Non-existent dir should return 0
        assert_eq!(count_source_files_in(&PathBuf::from("/tmp/nonexistent"), "rust"), 0);
    }

    #[test]
    fn test_false_positive_string_literal() {
        // Template generators that OUTPUT todo!() should not be flagged
        use super::scan_rust_stubs;
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        // This is what Forge's blueprint generator does — it creates skeleton code WITH todo!()
        std::fs::write(src.join("generator.rs"), r#"
fn generate_skeleton() -> String {
    let mut s = String::new();
    s.push_str("    todo!()\n");
    s
}
"#).unwrap();

        let mut gaps = Vec::new();
        scan_rust_stubs(&dir.path().to_path_buf(), "test", &mut gaps);
        assert!(gaps.is_empty(), "String literal todo!() should not be flagged, got: {:?}", gaps);
    }

    #[test]
    fn test_false_positive_assert() {
        use super::scan_rust_stubs;
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        // Test assertions checking for todo!() presence
        std::fs::write(src.join("test_gen.rs"), r#"
fn test_skeleton_has_stubs() {
    assert!(skeleton.contains("todo!()"));
}
"#).unwrap();

        let mut gaps = Vec::new();
        scan_rust_stubs(&dir.path().to_path_buf(), "test", &mut gaps);
        assert!(gaps.is_empty(), "Assert containing todo!() should not be flagged, got: {:?}", gaps);
    }

    #[test]
    fn test_false_positive_test_fixtures() {
        use super::scan_rust_stubs;
        let dir = tempfile::tempdir().unwrap();
        let tests_dir = dir.path().join("tests");
        std::fs::create_dir_all(&tests_dir).unwrap();

        // Test fixture with template string
        std::fs::write(tests_dir.join("edge_stress.rs"), r#"
const TEMPLATE: &str = "fn {{name}}() { todo!() }";
"#).unwrap();

        let mut gaps = Vec::new();
        scan_rust_stubs(&dir.path().to_path_buf(), "test", &mut gaps);
        assert!(gaps.is_empty(), "Test fixture todo!() should not be flagged, got: {:?}", gaps);
    }

    #[test]
    fn test_real_stub_still_flagged() {
        use super::scan_rust_stubs;
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        // This IS a real unimplemented stub
        std::fs::write(src.join("lib.rs"), r#"
fn actual_function() {
    todo!()
}

fn another_stub() -> Result<(), Error> {
    unimplemented!()
}
"#).unwrap();

        let mut gaps = Vec::new();
        scan_rust_stubs(&dir.path().to_path_buf(), "test", &mut gaps);
        assert_eq!(gaps.len(), 2, "Real stubs must still be flagged, got: {:?}", gaps);
    }

    #[test]
    fn test_comment_not_flagged() {
        use super::scan_rust_stubs;
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        // Comments mentioning todo!() are not gaps
        std::fs::write(src.join("scanner.rs"), r#"
// Scans for todo!() and unimplemented!() patterns in source code
/// Detects todo!() stubs that need implementation
"#).unwrap();

        let mut gaps = Vec::new();
        scan_rust_stubs(&dir.path().to_path_buf(), "test", &mut gaps);
        assert!(gaps.is_empty(), "Comments mentioning todo!() should not be flagged, got: {:?}", gaps);
    }
}
