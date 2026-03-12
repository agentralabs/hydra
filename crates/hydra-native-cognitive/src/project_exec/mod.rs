//! Autonomous Project Execution Engine — clone, understand, setup, test, report.
//!
//! Wires together all Phase 5 autonomy primitives:
//! - P1 SelfImplement — spec-driven code generation
//! - P2 Self-Unblocking — obstacle resolution
//! - P3 Environment — system awareness
//! - P4 Tool Discovery — dependency installation
//! - P5 Knowledge — on-the-fly learning
//!
//! The `ProjectExecutor` orchestrates end-to-end autonomous project evaluation.

pub mod analyzer;
pub mod cloner;
pub mod reporter;
pub mod setup;
pub mod tester;

pub use analyzer::ProjectAnalysis;
pub use cloner::CloneResult;
pub use reporter::{ProjectReport, ProjectStatus, ObstacleRecord};
pub use setup::CommandOutput;
pub use tester::TestResult;

use std::path::Path;
use tokio::sync::mpsc;
use crate::cognitive::loop_runner::CognitiveUpdate;
use crate::cognitive::obstacles::{Obstacle, ObstacleResolver, ResolverConfig};
use crate::environment::EnvironmentProfile;
use crate::task_persistence::{TaskCheckpoint, TaskType, TaskPersister};
use crate::tools::ToolInstaller;

/// Request to execute a project.
#[derive(Debug, Clone)]
pub struct ProjectRequest {
    pub repo_url: String,
    pub dry_run: bool,
    pub timeout_per_cmd: u64,
}

impl ProjectRequest {
    pub fn new(url: &str) -> Self {
        Self {
            repo_url: url.to_string(),
            dry_run: false,
            timeout_per_cmd: 300,
        }
    }

    pub fn dry_run(url: &str) -> Self {
        Self {
            repo_url: url.to_string(),
            dry_run: true,
            timeout_per_cmd: 300,
        }
    }
}

/// Autonomous project executor — coordinates all P1–P5 capabilities.
pub struct ProjectExecutor {
    #[allow(dead_code)]
    env: EnvironmentProfile,
    tools: ToolInstaller,
    resolver: ObstacleResolver,
    persister: TaskPersister,
}

impl ProjectExecutor {
    pub fn new() -> Self {
        let env = EnvironmentProfile::probe_all();
        let tools = ToolInstaller::new(env.clone());
        let resolver = ObstacleResolver::new(ResolverConfig::default());
        let persister = TaskPersister::new();
        Self { env, tools, resolver, persister }
    }

    /// Execute a full project evaluation pipeline.
    /// Sends progress updates via the tx channel.
    pub fn execute(
        &mut self,
        request: &ProjectRequest,
        tx: &mpsc::Sender<CognitiveUpdate>,
    ) -> ProjectReport {
        let repo_name = cloner::repo_name_from_url(&request.repo_url);
        let mut report = ProjectReport::new(&request.repo_url, &repo_name);
        let task_id = format!("project-exec-{}", repo_name);
        let mut checkpoint = TaskCheckpoint::new(
            &task_id, TaskType::ProjectExec,
            &["clone", "analyze", "setup", "test", "report"],
        );
        checkpoint.state = serde_json::json!({ "repo_url": &request.repo_url });
        let _ = self.persister.save(&checkpoint);

        // Phase 1: Clone
        send_phase(tx, &format!("Cloning {}...", repo_name));
        report.phase("clone");

        let work_dir = match cloner::create_work_dir() {
            Ok(d) => d,
            Err(e) => {
                report.status = ProjectStatus::CloneFailed;
                report.add_obstacle("clone", &e, false, None);
                report.finalize();
                return report;
            }
        };

        let clone_result = cloner::clone_repo(&request.repo_url, &work_dir);
        if !clone_result.success {
            report.status = ProjectStatus::CloneFailed;
            let err = clone_result.error.as_deref().unwrap_or("unknown error");
            report.add_obstacle("clone", err, false, None);
            report.finalize();
            return report;
        }

        let project_dir = &clone_result.project_dir;
        checkpoint.advance("clone", "analyzing", serde_json::json!({ "project_dir": &project_dir }));
        let _ = self.persister.save(&checkpoint);

        // Phase 2: Analyze/Understand
        send_phase(tx, "Reading documentation...");
        report.phase("analyze");
        let analysis = analyzer::analyze_project(project_dir);

        // Build LLM prompts for deeper understanding (caller can use these)
        let _learn_prompts = analyzer::build_learn_prompts(project_dir);

        report.analysis = Some(analysis.clone());
        send_msg(tx, &format!("Understood: {}", analysis.summary()));

        if request.dry_run {
            send_msg(tx, &format!(
                "**Dry run complete.**\nSetup: {}\nTests: {}",
                analysis.setup_commands().join(", "),
                analysis.test_commands().join(", ")
            ));
            report.finalize();
            return report;
        }

        checkpoint.advance("analyze", "setup", serde_json::json!({ "project_dir": &project_dir }));
        let _ = self.persister.save(&checkpoint);

        // Phase 3: Setup (with obstacle retry + tool install on failure)
        let setup_cmds = analysis.setup_commands();
        if !setup_cmds.is_empty() {
            send_phase(tx, &format!("Running setup ({} commands)...", setup_cmds.len()));
            report.phase("setup");
            let safe_cmds = setup::filter_safe_commands(&setup_cmds);
            for cmd in &safe_cmds {
                let result = setup::run_command(cmd, project_dir, request.timeout_per_cmd);
                send_msg(tx, &result.summary());
                if result.success {
                    report.setup_results.push(result);
                    continue;
                }
                let error_output = result.combined_output(500);
                // Try tool install if missing dependency detected
                let installed = self.try_auto_install(&error_output, tx);
                // Try obstacle resolver
                let obstacle = Obstacle::from_error(&error_output, cmd);
                let resolution = self.resolver.resolve_with_strategies(&obstacle, None, vec![]);
                if installed || resolution.is_fixed() {
                    send_msg(tx, &format!("Retrying: {}", cmd));
                    let retry = setup::run_command(cmd, project_dir, request.timeout_per_cmd);
                    send_msg(tx, &retry.summary());
                    report.add_obstacle("setup", &error_output, retry.success, None);
                    report.setup_results.push(retry);
                } else {
                    report.add_obstacle("setup", &error_output, false, None);
                    report.setup_results.push(result);
                }
            }
        }

        checkpoint.advance("setup", "testing", serde_json::json!({ "project_dir": &project_dir }));
        let _ = self.persister.save(&checkpoint);

        // Phase 4: Test (with obstacle retry + tool install on failure)
        let test_cmds = analysis.test_commands();
        if !test_cmds.is_empty() {
            send_phase(tx, &format!("Running tests ({} commands)...", test_cmds.len()));
            report.phase("test");
            for cmd in &test_cmds {
                if !setup::is_safe_command(cmd) { continue; }
                let result = tester::run_tests(cmd, project_dir, request.timeout_per_cmd);
                send_msg(tx, &result.summary());
                if result.success {
                    report.test_results.push(result);
                    continue;
                }
                let error_output = result.raw_output.combined_output(500);
                let installed = self.try_auto_install(&error_output, tx);
                let obstacle = Obstacle::from_error(&error_output, cmd);
                let resolution = self.resolver.resolve_with_strategies(&obstacle, None, vec![]);
                if installed || resolution.is_fixed() {
                    send_msg(tx, &format!("Retrying: {}", cmd));
                    let retry = tester::run_tests(cmd, project_dir, request.timeout_per_cmd);
                    send_msg(tx, &retry.summary());
                    report.add_obstacle("test", &error_output, retry.success, None);
                    report.test_results.push(retry);
                } else {
                    report.add_obstacle("test", &error_output, false, None);
                    report.test_results.push(result);
                }
            }
        }

        // Finalize
        report.finalize();
        checkpoint.mark_complete();
        let _ = self.persister.complete(&task_id);
        send_phase(tx, "Report ready");
        send_msg(tx, &report.summary());

        report
    }
}

impl ProjectExecutor {
    /// Detect and install a missing tool from error output. Returns true if installed.
    fn try_auto_install(&self, error: &str, tx: &mpsc::Sender<CognitiveUpdate>) -> bool {
        let tool = match self.tools.detect_missing(error) {
            Some(t) => t,
            None => return false,
        };
        let (cmd, pm) = match self.tools.build_install_command(&tool) {
            Ok(c) => c,
            Err(_) => return false,
        };
        send_msg(tx, &format!("Installing {} via {}...", tool.name, pm));
        let output = std::process::Command::new("sh").arg("-c").arg(&cmd).output();
        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let result = crate::tools::InstallResult {
                    success: true,
                    tool_name: tool.name.clone(),
                    package_manager: pm.clone(),
                    command_used: cmd.clone(),
                    output: stdout,
                    error: None,
                };
                // Store as belief for future reference
                let _belief = crate::tools::installer::install_as_belief(&tool, &result);
                send_msg(tx, &format!("Installed {} successfully", tool.name));
                true
            }
            _ => {
                send_msg(tx, &format!("Failed to install {}", tool.name));
                false
            }
        }
    }
}

impl ProjectExecutor {
    /// Access the task persister for listing/cancelling tasks.
    pub fn persister(&self) -> &TaskPersister {
        &self.persister
    }
}

impl Default for ProjectExecutor {
    fn default() -> Self { Self::new() }
}

/// Extract a URL from user text.
pub fn extract_url(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let w = word.trim_matches(|c: char| c == '<' || c == '>' || c == '"' || c == '\'');
        if w.starts_with("https://") || w.starts_with("http://") || w.starts_with("git@") {
            return Some(w.to_string());
        }
    }
    None
}

/// Check if user input looks like a project execution request.
pub fn is_project_exec_request(input: &str) -> bool {
    let lower = input.to_lowercase();
    let has_url = input.contains("github.com") || input.contains("gitlab.com") || input.contains("git@");
    let has_action = lower.contains("test") || lower.contains("evaluate") || lower.contains("clone and")
        || lower.contains("check") || lower.contains("run");
    has_url && has_action
}

fn send_phase(tx: &mpsc::Sender<CognitiveUpdate>, msg: &str) {
    let _ = tx.try_send(CognitiveUpdate::Phase(msg.to_string()));
}

fn send_msg(tx: &mpsc::Sender<CognitiveUpdate>, content: &str) {
    let _ = tx.try_send(CognitiveUpdate::Message {
        role: "system".to_string(),
        content: content.to_string(),
        css_class: "msg-system".to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_url() {
        assert_eq!(
            extract_url("test https://github.com/user/repo"),
            Some("https://github.com/user/repo".into())
        );
        assert_eq!(
            extract_url("evaluate <https://github.com/user/repo>"),
            Some("https://github.com/user/repo".into())
        );
        assert_eq!(extract_url("no url here"), None);
    }

    #[test]
    fn test_is_project_exec_request() {
        assert!(is_project_exec_request("test https://github.com/user/repo"));
        assert!(is_project_exec_request("evaluate this repo https://github.com/user/repo"));
        assert!(is_project_exec_request("clone and test https://github.com/user/repo"));
        assert!(!is_project_exec_request("what is the weather"));
        assert!(!is_project_exec_request("test my code")); // no URL
    }

    #[test]
    fn test_project_request_new() {
        let req = ProjectRequest::new("https://github.com/user/repo");
        assert!(!req.dry_run);
        assert_eq!(req.timeout_per_cmd, 300);
    }

    #[test]
    fn test_project_request_dry_run() {
        let req = ProjectRequest::dry_run("https://github.com/user/repo");
        assert!(req.dry_run);
    }

    #[test]
    fn test_executor_default() {
        let _exec = ProjectExecutor::default();
    }

    #[test]
    fn test_execute_dry_run() {
        let mut exec = ProjectExecutor::new();
        let (tx, _rx) = mpsc::channel(100);

        // Use the current project as a "repo" for dry run
        let root = std::env::current_dir().unwrap();
        let req = ProjectRequest {
            repo_url: root.to_str().unwrap().to_string(),
            dry_run: true,
            timeout_per_cmd: 10,
        };

        // Won't actually clone since it's a local path, but tests the flow
        let report = exec.execute(&req, &tx);
        // It will fail to clone (not a URL), but shouldn't panic
        assert!(report.repo_name.len() > 0);
    }
}
