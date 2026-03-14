//! Report generator — creates structured feedback reports from project execution.

use super::analyzer::ProjectAnalysis;
use super::setup::CommandOutput;
use super::tester::TestResult;

/// Overall status of a project execution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectStatus {
    Success,
    PartialSuccess,
    Failed,
    CloneFailed,
}

impl ProjectStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Success => "SUCCESS",
            Self::PartialSuccess => "PARTIAL",
            Self::Failed => "FAILED",
            Self::CloneFailed => "CLONE FAILED",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            Self::Success => "[OK]",
            Self::PartialSuccess => "[!!]",
            Self::Failed => "[FAIL]",
            Self::CloneFailed => "[FAIL]",
        }
    }
}

/// Full project execution report.
#[derive(Debug, Clone)]
pub struct ProjectReport {
    pub repo_url: String,
    pub repo_name: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub phases: Vec<String>,
    pub analysis: Option<ProjectAnalysis>,
    pub setup_results: Vec<CommandOutput>,
    pub test_results: Vec<TestResult>,
    pub obstacles: Vec<ObstacleRecord>,
    pub status: ProjectStatus,
}

/// Record of an obstacle encountered during execution.
#[derive(Debug, Clone)]
pub struct ObstacleRecord {
    pub phase: String,
    pub error: String,
    pub resolved: bool,
    pub resolution: Option<String>,
}

impl ProjectReport {
    /// Create a new report for a project execution.
    pub fn new(repo_url: &str, repo_name: &str) -> Self {
        Self {
            repo_url: repo_url.to_string(),
            repo_name: repo_name.to_string(),
            started_at: chrono::Utc::now(),
            completed_at: None,
            phases: Vec::new(),
            analysis: None,
            setup_results: Vec::new(),
            test_results: Vec::new(),
            obstacles: Vec::new(),
            status: ProjectStatus::Failed,
        }
    }

    /// Record a phase transition.
    pub fn phase(&mut self, name: &str) {
        self.phases.push(name.to_string());
    }

    /// Record an obstacle.
    pub fn add_obstacle(&mut self, phase: &str, error: &str, resolved: bool, resolution: Option<&str>) {
        self.obstacles.push(ObstacleRecord {
            phase: phase.to_string(),
            error: error.to_string(),
            resolved,
            resolution: resolution.map(String::from),
        });
    }

    /// Finalize the report — compute overall status.
    pub fn finalize(&mut self) {
        self.completed_at = Some(chrono::Utc::now());
        self.status = self.compute_status();
    }

    fn compute_status(&self) -> ProjectStatus {
        let all_setup_ok = self.setup_results.iter().all(|r| r.success);
        let all_tests_ok = self.test_results.iter().all(|r| r.success);
        let any_tests = !self.test_results.is_empty();

        if all_setup_ok && all_tests_ok && any_tests {
            ProjectStatus::Success
        } else if all_setup_ok && any_tests {
            ProjectStatus::PartialSuccess
        } else if all_setup_ok {
            ProjectStatus::PartialSuccess
        } else {
            ProjectStatus::Failed
        }
    }

    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("## {} — {}", self.repo_name, self.status.label()));
        lines.push(String::new());

        // Understanding
        if let Some(analysis) = &self.analysis {
            lines.push(format!("**Project:** {}", analysis.summary()));
        }

        // Setup
        let setup_ok = self.setup_results.iter().filter(|r| r.success).count();
        let setup_total = self.setup_results.len();
        if setup_total > 0 {
            lines.push(format!("**Setup:** {}/{} commands succeeded", setup_ok, setup_total));
        }

        // Tests
        let total_passed: usize = self.test_results.iter().map(|r| r.passed).sum();
        let total_failed: usize = self.test_results.iter().map(|r| r.failed).sum();
        let total_tests: usize = self.test_results.iter().map(|r| r.total).sum();
        if total_tests > 0 {
            lines.push(format!("**Tests:** {}/{} passed ({} failed)", total_passed, total_tests, total_failed));
        } else if !self.test_results.is_empty() {
            let ok = self.test_results.iter().filter(|r| r.success).count();
            lines.push(format!("**Tests:** {}/{} commands passed", ok, self.test_results.len()));
        }

        // Obstacles
        if !self.obstacles.is_empty() {
            let resolved = self.obstacles.iter().filter(|o| o.resolved).count();
            lines.push(format!(
                "**Obstacles:** {} encountered, {} resolved",
                self.obstacles.len(), resolved
            ));
        }

        // Duration
        if let Some(end) = self.completed_at {
            let dur = end - self.started_at;
            lines.push(format!("**Duration:** {:.0}s", dur.num_seconds()));
        }

        lines.join("\n")
    }

    /// One-line summary for memory storage (e.g., "repo: SUCCESS — 47/47 tests passed").
    pub fn one_line_summary(&self) -> String {
        let total_passed: usize = self.test_results.iter().map(|r| r.passed).sum();
        let total_tests: usize = self.test_results.iter().map(|r| r.total).sum();
        if total_tests > 0 {
            format!("{}: {} -- {}/{} tests passed", self.repo_name, self.status.label(), total_passed, total_tests)
        } else {
            format!("{}: {}", self.repo_name, self.status.label())
        }
    }

    /// Generate a detailed report table.
    pub fn detailed_table(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("# Project Report: {}", self.repo_name));
        lines.push(format!("URL: {}", self.repo_url));
        lines.push(format!("Status: {} {}", self.status.icon(), self.status.label()));
        lines.push(String::new());

        // Setup results
        if !self.setup_results.is_empty() {
            lines.push("## Setup".to_string());
            for r in &self.setup_results {
                lines.push(format!("  {}", r.summary()));
            }
            lines.push(String::new());
        }

        // Test results
        if !self.test_results.is_empty() {
            lines.push("## Tests".to_string());
            for r in &self.test_results {
                lines.push(format!("  {}", r.summary()));
            }
            lines.push(String::new());
        }

        // Obstacles
        if !self.obstacles.is_empty() {
            lines.push("## Obstacles".to_string());
            for o in &self.obstacles {
                let status = if o.resolved { "RESOLVED" } else { "UNRESOLVED" };
                lines.push(format!("  [{}] {}: {}", status, o.phase, o.error));
                if let Some(res) = &o.resolution {
                    lines.push(format!("    Fix: {}", res));
                }
            }
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_cmd_output(cmd: &str, success: bool) -> CommandOutput {
        CommandOutput {
            command: cmd.into(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: if success { 0 } else { 1 },
            success,
            duration: Duration::from_secs(1),
        }
    }

    fn make_test_result(cmd: &str, passed: usize, failed: usize) -> TestResult {
        TestResult {
            command: cmd.into(),
            passed, failed, ignored: 0,
            total: passed + failed,
            duration_secs: 1.0,
            success: failed == 0,
            raw_output: make_cmd_output(cmd, failed == 0),
        }
    }

    #[test]
    fn test_report_success() {
        let mut report = ProjectReport::new("https://github.com/user/repo", "repo");
        report.setup_results.push(make_cmd_output("cargo build", true));
        report.test_results.push(make_test_result("cargo test", 47, 0));
        report.finalize();
        assert_eq!(report.status, ProjectStatus::Success);
    }

    #[test]
    fn test_report_partial() {
        let mut report = ProjectReport::new("https://github.com/user/repo", "repo");
        report.setup_results.push(make_cmd_output("cargo build", true));
        report.test_results.push(make_test_result("cargo test", 40, 7));
        report.finalize();
        assert_eq!(report.status, ProjectStatus::PartialSuccess);
    }

    #[test]
    fn test_report_failed() {
        let mut report = ProjectReport::new("https://github.com/user/repo", "repo");
        report.setup_results.push(make_cmd_output("cargo build", false));
        report.finalize();
        assert_eq!(report.status, ProjectStatus::Failed);
    }

    #[test]
    fn test_report_summary() {
        let mut report = ProjectReport::new("https://github.com/user/repo", "repo");
        report.setup_results.push(make_cmd_output("cargo build", true));
        report.test_results.push(make_test_result("cargo test", 47, 0));
        report.finalize();
        let s = report.summary();
        assert!(s.contains("SUCCESS"));
        assert!(s.contains("47/47"));
    }

    #[test]
    fn test_report_detailed_table() {
        let mut report = ProjectReport::new("https://github.com/user/repo", "repo");
        report.setup_results.push(make_cmd_output("cargo build", true));
        report.test_results.push(make_test_result("cargo test", 47, 0));
        report.add_obstacle("setup", "missing gcc", true, Some("brew install gcc"));
        report.finalize();
        let table = report.detailed_table();
        assert!(table.contains("Project Report"));
        assert!(table.contains("Setup"));
        assert!(table.contains("Tests"));
        assert!(table.contains("Obstacles"));
        assert!(table.contains("RESOLVED"));
    }

    #[test]
    fn test_status_labels() {
        assert_eq!(ProjectStatus::Success.label(), "SUCCESS");
        assert_eq!(ProjectStatus::Failed.icon(), "[FAIL]");
        assert_eq!(ProjectStatus::PartialSuccess.icon(), "[!!]");
    }
}
