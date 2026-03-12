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
pub struct ProjectExecutor;

impl ProjectExecutor {
    pub fn new() -> Self { Self }

    /// Execute a full project evaluation pipeline.
    /// Sends progress updates via the tx channel.
    pub fn execute(
        &self,
        request: &ProjectRequest,
        tx: &mpsc::Sender<CognitiveUpdate>,
    ) -> ProjectReport {
        let repo_name = cloner::repo_name_from_url(&request.repo_url);
        let mut report = ProjectReport::new(&request.repo_url, &repo_name);

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

        // Phase 3: Setup
        let setup_cmds = analysis.setup_commands();
        if !setup_cmds.is_empty() {
            send_phase(tx, &format!("Running setup ({} commands)...", setup_cmds.len()));
            report.phase("setup");

            let safe_cmds = setup::filter_safe_commands(&setup_cmds);
            let results = setup::run_setup_commands(
                &safe_cmds, project_dir, request.timeout_per_cmd, false,
            );

            for result in &results {
                send_msg(tx, &result.summary());
                if !result.success {
                    report.add_obstacle(
                        "setup",
                        &result.combined_output(500),
                        false,
                        None,
                    );
                }
            }
            report.setup_results = results;
        }

        // Phase 4: Test
        let test_cmds = analysis.test_commands();
        if !test_cmds.is_empty() {
            send_phase(tx, &format!("Running tests ({} commands)...", test_cmds.len()));
            report.phase("test");

            for cmd in &test_cmds {
                if !setup::is_safe_command(cmd) {
                    continue;
                }
                let result = tester::run_tests(cmd, project_dir, request.timeout_per_cmd);
                send_msg(tx, &result.summary());
                if !result.success {
                    report.add_obstacle(
                        "test",
                        &result.raw_output.combined_output(500),
                        false,
                        None,
                    );
                }
                report.test_results.push(result);
            }
        }

        // Finalize
        report.finalize();
        send_phase(tx, "Report ready");
        send_msg(tx, &report.summary());

        report
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
        let exec = ProjectExecutor::new();
        let (tx, mut rx) = mpsc::channel(100);

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
