//! Phased build plan for project creation.
//!
//! Decomposes the project into ordered phases. Each phase targets one file,
//! compiles before the next starts, and accumulates context from prior phases.
//! Checkpoints to disk for resume after interruption or rate limiting.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use super::project_creation::ProjectConfig;

/// A single build phase — one LLM call, one file, one compile check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildPhase {
    pub index: usize,
    pub name: String,
    pub phase_type: PhaseType,
    /// Primary file this phase modifies/creates.
    pub target_file: String,
    /// Additional files to update (e.g., mod.rs when adding a new module).
    pub secondary_files: Vec<String>,
    /// Description sent to LLM — what to generate.
    pub description: String,
    pub requires_llm: bool,
    pub status: PhaseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PhaseType {
    Scaffold,
    CoreTypes,
    ToolImpl(String),
    IntegrationTest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Compiled,
    Failed(String),
    Skipped,
}

/// The full build plan with checkpoint support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildPlan {
    pub project_name: String,
    pub project_key: String,
    pub project_dir: PathBuf,
    pub phases: Vec<BuildPhase>,
    pub current_phase: usize,
}

impl BuildPlan {
    /// Generate a build plan from a ProjectConfig.
    /// Deterministic — no LLM needed. Handles any number of tools.
    pub fn from_config(config: &ProjectConfig) -> Self {
        let mut phases = Vec::new();
        let key = &config.key;
        let key_under = key.replace('-', "_");

        // Phase 0: Scaffold (no LLM — templates only)
        phases.push(BuildPhase {
            index: 0,
            name: "Scaffold workspace".into(),
            phase_type: PhaseType::Scaffold,
            target_file: String::new(),
            secondary_files: vec![],
            description: "Create Cargo workspace with all crate skeletons".into(),
            requires_llm: false,
            status: PhaseStatus::Pending,
        });

        // Phase 1: Core types + storage engine
        phases.push(BuildPhase {
            index: 1,
            name: "Core types & storage".into(),
            phase_type: PhaseType::CoreTypes,
            target_file: format!("crates/agentic-{}/src/lib.rs", key),
            secondary_files: vec![],
            description: format!(
                "Enhance the core Store for {} with domain-specific types and methods. \
                 The scaffold has basic CRUD — add any domain logic the spec requires.",
                config.name
            ),
            requires_llm: true,
            status: PhaseStatus::Pending,
        });

        // Phases 2..N: One tool at a time
        // Each phase ENHANCES registry.rs (adds/replaces one handler).
        // For >10 tools, creates separate files + updates mod.rs.
        let use_separate_files = config.tools.len() > 10;

        for (i, tool) in config.tools.iter().enumerate() {
            let (target, secondaries, desc) = if use_separate_files {
                // Large project: separate file per tool + mod.rs + registry.rs wiring
                let tool_file = format!(
                    "crates/agentic-{}-mcp/src/tools/{}.rs", key, tool
                );
                let mod_file = format!(
                    "crates/agentic-{}-mcp/src/tools/mod.rs", key
                );
                let reg_file = format!(
                    "crates/agentic-{}-mcp/src/tools/registry.rs", key
                );
                (
                    tool_file,
                    vec![mod_file, reg_file],
                    format!(
                        "Create tools/{tool}.rs with handle_{tool}(params, store) -> Result. \
                         Also add `pub mod {tool};` to tools/mod.rs and wire dispatch in registry.rs. \
                         The handler uses agentic_{key}::Store methods.",
                        tool = tool, key = key_under
                    ),
                )
            } else {
                // Small project: all handlers in registry.rs (scaffold already did this)
                let reg_file = format!(
                    "crates/agentic-{}-mcp/src/tools/registry.rs", key
                );
                (
                    reg_file,
                    vec![],
                    format!(
                        "Enhance handle_{tool} in registry.rs with domain-specific logic \
                         from the spec. The scaffold has a basic implementation — make it \
                         match what the spec requires.",
                        tool = tool
                    ),
                )
            };

            phases.push(BuildPhase {
                index: 2 + i,
                name: format!("Tool: {}", tool),
                phase_type: PhaseType::ToolImpl(tool.clone()),
                target_file: target,
                secondary_files: secondaries,
                description: desc,
                requires_llm: true,
                status: PhaseStatus::Pending,
            });
        }

        // Final: Integration test
        let test_idx = 2 + config.tools.len();
        phases.push(BuildPhase {
            index: test_idx,
            name: "Integration tests".into(),
            phase_type: PhaseType::IntegrationTest,
            target_file: format!("crates/agentic-{}-mcp/src/tools/registry.rs", key),
            secondary_files: vec![],
            description: "Add integration tests exercising the full MCP request/response \
                         cycle for each tool via handle_request().".into(),
            requires_llm: true,
            status: PhaseStatus::Pending,
        });

        BuildPlan {
            project_name: config.name.clone(),
            project_key: config.key.clone(),
            project_dir: config.target_dir.clone(),
            phases,
            current_phase: 0,
        }
    }

    /// Save checkpoint to disk for resume after interruption.
    pub fn save_checkpoint(&self) -> Result<(), String> {
        let path = self.project_dir.join(".hydra-build-plan.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialize build plan: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("Write build plan: {}", e))
    }

    /// Load checkpoint from disk. Returns None if no checkpoint exists.
    pub fn load_checkpoint(project_dir: &Path) -> Option<Self> {
        let path = project_dir.join(".hydra-build-plan.json");
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Get the next pending phase, or None if all done.
    pub fn next_phase(&self) -> Option<&BuildPhase> {
        self.phases.iter().find(|p| p.status == PhaseStatus::Pending)
    }

    pub fn mark_compiled(&mut self, index: usize) {
        if let Some(phase) = self.phases.get_mut(index) {
            phase.status = PhaseStatus::Compiled;
            self.current_phase = index + 1;
        }
    }

    pub fn mark_failed(&mut self, index: usize, error: String) {
        if let Some(phase) = self.phases.get_mut(index) {
            phase.status = PhaseStatus::Failed(error);
        }
    }

    pub fn mark_skipped(&mut self, index: usize) {
        if let Some(phase) = self.phases.get_mut(index) {
            phase.status = PhaseStatus::Skipped;
            self.current_phase = index + 1;
        }
    }

    /// Skip ALL remaining pending phases.
    pub fn skip_all_remaining(&mut self) {
        for phase in &mut self.phases {
            if phase.status == PhaseStatus::Pending {
                phase.status = PhaseStatus::Skipped;
            }
        }
        self.current_phase = self.phases.len();
    }

    /// Progress: "3/8 phases complete"
    pub fn progress_summary(&self) -> String {
        let done = self.phases.iter()
            .filter(|p| matches!(p.status, PhaseStatus::Compiled | PhaseStatus::Skipped))
            .count();
        let failed = self.phases.iter()
            .filter(|p| matches!(p.status, PhaseStatus::Failed(_)))
            .count();
        let total = self.phases.len();
        if failed > 0 {
            format!("{}/{} phases complete, {} failed", done, total, failed)
        } else {
            format!("{}/{} phases complete", done, total)
        }
    }

    /// Gather actual compiled code from completed phases as LLM context.
    pub fn gather_completed_context(&self) -> String {
        let mut ctx = String::new();
        let mut seen = std::collections::HashSet::new();
        for phase in &self.phases {
            if !matches!(phase.status, PhaseStatus::Compiled) || phase.target_file.is_empty() {
                continue;
            }
            if !seen.insert(phase.target_file.clone()) { continue; }
            let path = self.project_dir.join(&phase.target_file);
            if let Ok(content) = std::fs::read_to_string(&path) {
                let truncated = &content[..content.len().min(1500)];
                ctx.push_str(&format!(
                    "### {} (compiled)\n```rust\n{}\n```\n\n",
                    phase.target_file, truncated
                ));
            }
        }
        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_plan_5_tools() {
        let config = ProjectConfig {
            key: "echo".into(), name: "AgenticEcho".into(),
            file_ext: ".aecho".into(), cli_binary: "aecho".into(),
            tools: vec![
                "echo_send".into(), "echo_query".into(),
                "echo_history".into(), "echo_stats".into(),
                "echo_clear".into(),
            ],
            description: "Test".into(), target_dir: "/tmp/agentic-echo".into(),
        };
        let plan = BuildPlan::from_config(&config);
        // scaffold + core + 5 tools + integration = 8
        assert_eq!(plan.phases.len(), 8);
        assert_eq!(plan.phases[0].phase_type, PhaseType::Scaffold);
        assert_eq!(plan.phases[1].phase_type, PhaseType::CoreTypes);
        assert_eq!(plan.phases[2].phase_type, PhaseType::ToolImpl("echo_send".into()));
        // 5 tools → ≤10 → all target registry.rs (no separate files)
        assert!(plan.phases[2].target_file.contains("registry.rs"));
        assert!(plan.phases[2].secondary_files.is_empty());
        assert!(matches!(plan.phases[7].phase_type, PhaseType::IntegrationTest));
    }

    #[test]
    fn test_large_project_separate_files() {
        let tools: Vec<String> = (0..12).map(|i| format!("data_op{}", i)).collect();
        let config = ProjectConfig {
            key: "data".into(), name: "AgenticData".into(),
            file_ext: ".adat".into(), cli_binary: "adat".into(),
            tools, description: "Test".into(), target_dir: "/tmp/agentic-data".into(),
        };
        let plan = BuildPlan::from_config(&config);
        // >10 tools → separate files
        assert!(plan.phases[2].target_file.contains("data_op0.rs"));
        assert_eq!(plan.phases[2].secondary_files.len(), 2); // mod.rs + registry.rs
    }

    #[test]
    fn test_skip_all_remaining() {
        let config = ProjectConfig {
            key: "t".into(), name: "T".into(), file_ext: ".t".into(),
            cli_binary: "t".into(), tools: vec!["t_a".into()],
            description: "t".into(), target_dir: "/tmp/t".into(),
        };
        let mut plan = BuildPlan::from_config(&config);
        plan.mark_compiled(0);
        plan.skip_all_remaining();
        assert!(plan.next_phase().is_none());
        assert!(plan.progress_summary().contains(&format!("{}", plan.phases.len())));
    }

    #[test]
    fn test_checkpoint_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let config = ProjectConfig {
            key: "rt".into(), name: "RT".into(), file_ext: ".rt".into(),
            cli_binary: "rt".into(), tools: vec!["rt_x".into()],
            description: "rt".into(), target_dir: tmp.path().to_path_buf(),
        };
        let mut plan = BuildPlan::from_config(&config);
        plan.mark_compiled(0);
        plan.save_checkpoint().unwrap();
        let loaded = BuildPlan::load_checkpoint(tmp.path()).unwrap();
        assert_eq!(loaded.current_phase, 1);
        assert_eq!(loaded.phases[0].status, PhaseStatus::Compiled);
    }
}
