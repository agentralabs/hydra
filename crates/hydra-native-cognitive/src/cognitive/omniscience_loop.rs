//! Omniscience orchestration — extracted from omniscience.rs for file size.
//! Contains apply_fix() and run_omniscience_loop().

use tokio::sync::mpsc;

use crate::sisters::Sisters;
use super::omniscience::{
    OmniscienceEngine, OmniscienceGap, OmniscienceScan, OmniscienceUpdate,
    RepoScan, RepoTarget,
};
use super::omniscience_scanners::{calculate_health_score, count_source_files_in};

impl OmniscienceEngine {
    /// Phase 5: Apply a fix using the Codebase→Forge→Aegis pipeline.
    ///
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

        let code_context = codebase.call_tool("search_semantic", serde_json::json!({
            "query": format!("Read and analyze {}: what does this file do, what are its dependencies, what calls into it", full_path),
        })).await.ok();

        let impact = codebase.call_tool("impact_analyze", serde_json::json!({
            "query": format!("What would be affected if we modify {} at the gap: {}", file_path, gap.description),
        })).await.ok();

        // STEP 2: PLAN — Generate a blueprint for the fix
        let blueprint = if let Some(forge) = forge {
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
}
