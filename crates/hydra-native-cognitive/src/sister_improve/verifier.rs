//! Test runner and verification for sister improvements.
//!
//! Runs tests before and after patching, compares results, and determines
//! whether the improvement is safe to keep.

use std::path::Path;
use super::analyzer::SisterAnalysis;

/// Parsed test results from running a sister's test suite.
#[derive(Debug, Clone)]
pub struct TestResults {
    pub pass_count: usize,
    pub fail_count: usize,
    pub skip_count: usize,
    pub total: usize,
    pub raw_output: String,
    pub duration_secs: f64,
}

impl TestResults {
    pub fn empty() -> Self {
        Self {
            pass_count: 0,
            fail_count: 0,
            skip_count: 0,
            total: 0,
            raw_output: String::new(),
            duration_secs: 0.0,
        }
    }

    /// Parse cargo test output.
    pub fn parse_cargo(output: &str) -> Self {
        let mut pass = 0;
        let mut fail = 0;
        let mut skip = 0;

        for line in output.lines() {
            let trimmed = line.trim();
            // "test result: ok. 15 passed; 0 failed; 2 ignored; ..."
            if trimmed.starts_with("test result:") {
                if let Some(p) = extract_count(trimmed, "passed") { pass += p; }
                if let Some(f) = extract_count(trimmed, "failed") { fail += f; }
                if let Some(i) = extract_count(trimmed, "ignored") { skip += i; }
            }
        }

        Self {
            pass_count: pass,
            fail_count: fail,
            skip_count: skip,
            total: pass + fail + skip,
            raw_output: output.to_string(),
            duration_secs: 0.0,
        }
    }

    /// Parse npm test / jest output.
    pub fn parse_npm(output: &str) -> Self {
        let mut pass = 0;
        let mut fail = 0;

        for line in output.lines() {
            let trimmed = line.trim();
            // "Tests: 5 passed, 1 failed, 6 total"
            if trimmed.contains("passed") && trimmed.contains("total") {
                if let Some(p) = extract_count(trimmed, "passed") { pass = p; }
                if let Some(f) = extract_count(trimmed, "failed") { fail = f; }
            }
        }

        Self {
            pass_count: pass,
            fail_count: fail,
            skip_count: 0,
            total: pass + fail,
            raw_output: output.to_string(),
            duration_secs: 0.0,
        }
    }

    /// Parse pytest output.
    pub fn parse_pytest(output: &str) -> Self {
        let mut pass = 0;
        let mut fail = 0;
        let mut skip = 0;

        for line in output.lines() {
            let trimmed = line.trim();
            // "=== 5 passed, 1 failed, 2 skipped ==="
            if trimmed.contains("passed") || trimmed.contains("failed") {
                if let Some(p) = extract_count(trimmed, "passed") { pass = p; }
                if let Some(f) = extract_count(trimmed, "failed") { fail = f; }
                if let Some(s) = extract_count(trimmed, "skipped") { skip = s; }
            }
        }

        Self {
            pass_count: pass,
            fail_count: fail,
            skip_count: skip,
            total: pass + fail + skip,
            raw_output: output.to_string(),
            duration_secs: 0.0,
        }
    }

    /// Parse go test output.
    pub fn parse_go(output: &str) -> Self {
        let mut pass = 0;
        let mut fail = 0;

        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("ok") { pass += 1; }
            if trimmed.starts_with("FAIL") { fail += 1; }
        }

        Self {
            pass_count: pass,
            fail_count: fail,
            skip_count: 0,
            total: pass + fail,
            raw_output: output.to_string(),
            duration_secs: 0.0,
        }
    }
}

/// Result of comparing before/after test runs.
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationResult {
    /// More tests pass, none regressed.
    Improved,
    /// Same pass count, no regressions.
    Neutral,
    /// Fewer tests pass or new failures.
    Regressed,
}

/// Compare baseline and after test results.
pub fn verify(baseline: &TestResults, after: &TestResults) -> VerificationResult {
    if after.fail_count > baseline.fail_count {
        return VerificationResult::Regressed;
    }
    if after.pass_count < baseline.pass_count {
        return VerificationResult::Regressed;
    }
    if after.pass_count > baseline.pass_count {
        return VerificationResult::Improved;
    }
    VerificationResult::Neutral
}

/// Run tests for a sister project.
pub async fn run_tests(sister_path: &Path, analysis: &SisterAnalysis) -> TestResults {
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&analysis.test_command)
        .current_dir(sister_path)
        .output()
        .await;

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            let combined = format!("{}\n{}", stdout, stderr);

            use super::analyzer::SisterLanguage;
            match &analysis.language {
                SisterLanguage::Rust => TestResults::parse_cargo(&combined),
                SisterLanguage::TypeScript => TestResults::parse_npm(&combined),
                SisterLanguage::Python => TestResults::parse_pytest(&combined),
                SisterLanguage::Go => TestResults::parse_go(&combined),
                SisterLanguage::Unknown(_) => {
                    // Best-effort: check exit code
                    TestResults {
                        pass_count: if o.status.success() { 1 } else { 0 },
                        fail_count: if o.status.success() { 0 } else { 1 },
                        skip_count: 0,
                        total: 1,
                        raw_output: combined,
                        duration_secs: 0.0,
                    }
                }
            }
        }
        Err(e) => {
            TestResults {
                pass_count: 0,
                fail_count: 0,
                skip_count: 0,
                total: 0,
                raw_output: format!("Failed to run tests: {}", e),
                duration_secs: 0.0,
            }
        }
    }
}

/// Extract a count before a keyword like "15 passed".
fn extract_count(line: &str, keyword: &str) -> Option<usize> {
    let idx = line.find(keyword)?;
    let before = &line[..idx].trim_end();
    let num_str = before.rsplit(|c: char| !c.is_ascii_digit()).next()?;
    num_str.parse().ok()
}
