//! Omniscience phases — extracted from omniscience.rs for file size.
//! Contains analyze_repo (Phase 1), generate_repo_specs (Phase 2),
//! validate_fix (Phase 3), identify_gaps, and generate_checks_for_gap.

use crate::cognitive::self_repair::{AcceptanceCheck, RepairSpec};
use crate::sisters::Sisters;
use super::omniscience::{OmniscienceEngine, OmniscienceGap, RepoTarget};
use super::omniscience_scanners::{scan_rust_stubs, scan_ts_stubs};

impl OmniscienceEngine {
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
            "command": format!("{} after applying: {}", check_cmd, hydra_native_state::utils::safe_truncate(&spec.instructions_for_claude_code, 200)),
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

    /// Identify gaps in a specific repo from analysis text + filesystem scan.
    pub(crate) fn identify_gaps(&self, target: &RepoTarget, analysis: &str) -> Vec<OmniscienceGap> {
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
    pub(crate) fn generate_checks_for_gap(&self, target: &RepoTarget, gap: &OmniscienceGap) -> Vec<AcceptanceCheck> {
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
