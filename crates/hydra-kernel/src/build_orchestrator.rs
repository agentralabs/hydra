//! Build orchestrator — sequences scaffold, implement, test, verify phases.
//!
//! Drives the full build lifecycle: plan -> scaffold new crates -> implement
//! in ordered batches -> test -> verify file sizes.

use std::path::PathBuf;
use std::time::Instant;

use crate::build_planner::{BuildPlan, CrateSpec, ImplementationStep};
use crate::cargo_ops::{CargoOps, CrateType};
use crate::self_modify::{ModResult, Patch, SpecGap};

/// Current state of the build.
#[derive(Debug, Clone)]
pub enum BuildState {
    Planning,
    Scaffolding { completed: usize, total: usize },
    Implementing { batch: usize, total: usize, patches_applied: usize },
    Testing { passed: usize, failed: usize },
    Verifying,
    Complete(BuildReport),
    Failed { phase: String, error: String, can_retry: bool },
}

/// Final report after build completes.
#[derive(Debug, Clone)]
pub struct BuildReport {
    pub crates_created: usize,
    pub files_modified: usize,
    pub patches_applied: usize,
    pub tests_passing: usize,
    pub tests_failed: usize,
    pub batches_completed: usize,
    pub retries_used: usize,
    pub duration_ms: u64,
}

/// The multi-phase build orchestrator.
pub struct BuildOrchestrator {
    project_dir: PathBuf,
    spec: String,
    plan: BuildPlan,
    state: BuildState,
    start_time: Instant,
    crates_created: usize,
    files_modified: usize,
    patches_applied: usize,
    retries_used: usize,
}

impl BuildOrchestrator {
    /// Create an orchestrator from an already-generated build plan.
    pub fn new(project_dir: PathBuf, spec: String, plan: BuildPlan) -> Self {
        Self {
            project_dir,
            spec,
            plan,
            state: BuildState::Planning,
            start_time: Instant::now(),
            crates_created: 0,
            files_modified: 0,
            patches_applied: 0,
            retries_used: 0,
        }
    }

    pub fn state(&self) -> &BuildState { &self.state }
    pub fn plan(&self) -> &BuildPlan { &self.plan }

    /// Execute Phase 1: Scaffold new crates if needed.
    pub fn execute_scaffold(&mut self) -> Result<(), String> {
        let new_crates: Vec<CrateSpec> = self.plan.crates.iter()
            .filter(|c| c.is_new)
            .cloned()
            .collect();

        if new_crates.is_empty() {
            self.state = BuildState::Scaffolding { completed: 0, total: 0 };
            return Ok(());
        }

        let total = new_crates.len();
        self.state = BuildState::Scaffolding { completed: 0, total };

        for (i, crate_spec) in new_crates.iter().enumerate() {
            let crate_type = match crate_spec.crate_type.as_str() {
                "bin" => CrateType::Bin,
                "both" => CrateType::Both,
                _ => CrateType::Lib,
            };

            let deps: Vec<(&str, &str)> = crate_spec.dependencies.iter()
                .map(|(n, s)| (n.as_str(), s.as_str()))
                .collect();

            CargoOps::scaffold_and_register(
                &self.project_dir,
                &crate_spec.name,
                crate_type,
                &crate_spec.description,
                &deps,
            )?;

            self.crates_created += 1;
            self.state = BuildState::Scaffolding { completed: i + 1, total };
            eprintln!("[hydra:build] Scaffolded crate '{}'", crate_spec.name);
        }

        Ok(())
    }

    /// Execute Phase 2: Implement code in batches.
    pub async fn execute_implement(
        &mut self,
        llm_config: &hydra_model::LlmConfig,
    ) -> Result<usize, String> {
        let steps = self.plan.implementation_order.clone();
        let total_batches = steps.len();
        let mut total_patches = 0;

        for (batch_idx, step) in steps.iter().enumerate() {
            self.state = BuildState::Implementing {
                batch: batch_idx + 1,
                total: total_batches,
                patches_applied: total_patches,
            };

            eprintln!(
                "[hydra:build] Batch {}/{}: {} ({})",
                batch_idx + 1, total_batches, step.description, step.crate_name
            );

            let batch_spec = build_batch_spec(step, &self.spec);

            let gaps = crate::self_modify_llm::analyze_spec_gaps(
                &batch_spec, None, llm_config, &self.project_dir,
            ).await.map_err(|e| format!("Batch {} gap analysis failed: {}", batch_idx + 1, e))?;

            if gaps.is_empty() {
                eprintln!("[hydra:build] Batch {}: no gaps found, skipping", batch_idx + 1);
                continue;
            }

            let patches = crate::self_modify_llm::generate_patches(
                &gaps, &batch_spec, None, llm_config, &self.project_dir,
            ).await.map_err(|e| format!("Batch {} patch gen failed: {}", batch_idx + 1, e))?;

            match self.apply_batch_with_retry(gaps, patches, &batch_spec, llm_config).await {
                Ok(count) => {
                    total_patches += count;
                    self.patches_applied = total_patches;
                    eprintln!("[hydra:build] Batch {}: {} patches applied", batch_idx + 1, count);
                }
                Err(e) => {
                    self.state = BuildState::Failed {
                        phase: format!("Implement batch {}", batch_idx + 1),
                        error: e.clone(),
                        can_retry: true,
                    };
                    return Err(e);
                }
            }
        }

        Ok(total_patches)
    }

    /// Apply a batch of patches with retry on compile failure.
    async fn apply_batch_with_retry(
        &mut self,
        gaps: Vec<SpecGap>,
        mut patches: Vec<Patch>,
        batch_spec: &str,
        llm_config: &hydra_model::LlmConfig,
    ) -> Result<usize, String> {
        let max_retries = 3;

        for attempt in 1..=max_retries {
            let pipeline = crate::self_modify::SelfModificationPipeline::new(&self.project_dir);
            let result = pipeline.run_from_gaps(gaps.clone(), patches.clone());

            match result {
                ModResult::Success { patches_applied, .. } => {
                    self.files_modified += patches_applied;
                    return Ok(patches_applied);
                }
                ModResult::CompileFailed { ref error, .. } if attempt < max_retries => {
                    eprintln!("[hydra:build] Attempt {}/{} failed, retrying...", attempt, max_retries);
                    self.retries_used += 1;
                    match crate::self_modify_llm::fix_compile_errors(
                        &patches, error, batch_spec, llm_config, &self.project_dir,
                    ).await {
                        Ok(fixed) => { patches = fixed; }
                        Err(e) => return Err(format!("Error correction failed: {}", e)),
                    }
                }
                ModResult::CompileFailed { error, .. } => {
                    return Err(format!("Compile failed after {} attempts: {}", max_retries, error));
                }
                other => {
                    return Err(format!("Pipeline error: {}", other.summary()));
                }
            }
        }

        Err("Max retries exhausted".into())
    }

    /// Execute Phase 3: Run tests on affected crates.
    pub fn execute_tests(&mut self) -> Result<(usize, usize), String> {
        let affected_crates: Vec<String> = self.plan.crates.iter()
            .map(|c| c.name.clone())
            .collect();

        let mut passed = 0usize;
        let mut failed = 0usize;

        for crate_name in &affected_crates {
            let output = std::process::Command::new("cargo")
                .args(["test", "-p", crate_name, "-j", "1", "--no-fail-fast"])
                .current_dir(&self.project_dir)
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let combined = format!("{}{}", stdout, stderr);
                    if let Some(count) = extract_test_count(&combined) {
                        passed += count;
                    }
                    eprintln!("[hydra:build] Tests passed for '{}'", crate_name);
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    failed += 1;
                    eprintln!(
                        "[hydra:build] Tests FAILED for '{}': {}",
                        crate_name, &stderr[..stderr.len().min(200)]
                    );
                }
                Err(e) => {
                    eprintln!("[hydra:build] Cannot run tests for '{}': {}", crate_name, e);
                }
            }
        }

        self.state = BuildState::Testing { passed, failed };
        Ok((passed, failed))
    }

    /// Execute Phase 4: Verify file sizes and module registration.
    pub fn execute_verify(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        self.state = BuildState::Verifying;

        for step in &self.plan.implementation_order {
            for file in &step.files {
                let path = self.project_dir.join("crates").join(&step.crate_name).join(file);
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let lines = content.lines().count();
                    if lines > 400 {
                        warnings.push(format!("{}: {} lines (max 400)", file, lines));
                    }
                }
            }
        }

        warnings
    }

    /// Generate the final build report.
    pub fn finalize(&mut self) -> BuildReport {
        let report = BuildReport {
            crates_created: self.crates_created,
            files_modified: self.files_modified,
            patches_applied: self.patches_applied,
            tests_passing: 0,
            tests_failed: 0,
            batches_completed: self.plan.implementation_order.len(),
            retries_used: self.retries_used,
            duration_ms: self.start_time.elapsed().as_millis() as u64,
        };
        self.state = BuildState::Complete(report.clone());
        report
    }

    /// Run all phases sequentially.
    pub async fn run_all(
        &mut self,
        llm_config: &hydra_model::LlmConfig,
    ) -> Result<BuildReport, String> {
        self.execute_scaffold()?;
        self.execute_implement(llm_config).await?;
        let (passed, failed) = self.execute_tests().unwrap_or((0, 0));
        let warnings = self.execute_verify();
        if !warnings.is_empty() {
            eprintln!("[hydra:build] Verify warnings: {:?}", warnings);
        }
        let mut report = self.finalize();
        report.tests_passing = passed;
        report.tests_failed = failed;
        Ok(report)
    }
}

/// Build a focused sub-spec for a single implementation batch.
fn build_batch_spec(step: &ImplementationStep, full_spec: &str) -> String {
    let truncated = &full_spec[..full_spec.len().min(3000)];
    format!(
        "Implement the following for crate '{}':\n{}\n\nTarget files: {}\n\nFull spec context:\n{}",
        step.crate_name,
        step.description,
        step.files.join(", "),
        truncated,
    )
}

/// Extract test count from cargo test output.
fn extract_test_count(output: &str) -> Option<usize> {
    for line in output.lines() {
        if line.contains("test result:") && line.contains("passed") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if (*part == "passed" || *part == "passed;") && i > 0 {
                    return parts[i - 1].trim_end_matches('.').parse().ok();
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_test_count() {
        let output = "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out";
        assert_eq!(extract_test_count(output), Some(5));
    }

    #[test]
    fn test_extract_test_count_multiline() {
        let output = "running 3 tests\ntest foo ... ok\ntest bar ... ok\ntest baz ... ok\n\ntest result: ok. 3 passed; 0 failed; 0 ignored";
        assert_eq!(extract_test_count(output), Some(3));
    }

    #[test]
    fn test_extract_test_count_none() {
        assert_eq!(extract_test_count("no test output here"), None);
        assert_eq!(extract_test_count("compiling hydra-kernel v0.1.0"), None);
    }

    #[test]
    fn test_build_report_defaults() {
        let report = BuildReport {
            crates_created: 0,
            files_modified: 0,
            patches_applied: 0,
            tests_passing: 0,
            tests_failed: 0,
            batches_completed: 0,
            retries_used: 0,
            duration_ms: 0,
        };
        assert_eq!(report.crates_created, 0);
        assert_eq!(report.duration_ms, 0);
    }

    #[test]
    fn test_build_batch_spec() {
        let step = ImplementationStep {
            crate_name: "hydra-kernel".to_string(),
            files: vec!["src/foo.rs".to_string(), "src/bar.rs".to_string()],
            description: "Add foo and bar".to_string(),
        };
        let spec = "Full spec content here";
        let result = build_batch_spec(&step, spec);
        assert!(result.contains("hydra-kernel"));
        assert!(result.contains("Add foo and bar"));
        assert!(result.contains("src/foo.rs, src/bar.rs"));
        assert!(result.contains("Full spec content here"));
    }
}
