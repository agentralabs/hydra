//! Test runner — executes test commands and parses results.

use std::path::Path;
use super::setup::{run_command, CommandOutput};

/// Parsed test result.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub command: String,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub total: usize,
    pub duration_secs: f64,
    pub success: bool,
    pub raw_output: CommandOutput,
}

impl TestResult {
    /// One-line summary.
    pub fn summary(&self) -> String {
        if self.total > 0 {
            format!(
                "{}: {}/{} passed ({} failed, {} ignored) in {:.1}s",
                self.command, self.passed, self.total,
                self.failed, self.ignored, self.duration_secs
            )
        } else {
            format!("{}: {}", self.command, if self.success { "passed" } else { "failed" })
        }
    }
}

/// Run a test command and parse the results.
pub fn run_tests(cmd: &str, dir: &Path, timeout_secs: u64) -> TestResult {
    let output = run_command(cmd, dir, timeout_secs);
    let combined = format!("{}\n{}", output.stdout, output.stderr);

    let (passed, failed, ignored) = parse_test_counts(&combined);
    let total = passed + failed + ignored;

    TestResult {
        command: cmd.to_string(),
        passed,
        failed,
        ignored,
        total,
        duration_secs: output.duration.as_secs_f64(),
        success: output.success,
        raw_output: output,
    }
}

/// Parse test counts from output. Supports multiple frameworks.
fn parse_test_counts(output: &str) -> (usize, usize, usize) {
    // Rust: "test result: ok. 47 passed; 0 failed; 2 ignored; ..."
    if let Some(counts) = parse_rust_test_output(output) {
        return counts;
    }

    // Jest/Vitest: "Tests: 3 failed, 47 passed, 50 total"
    if let Some(counts) = parse_jest_output(output) {
        return counts;
    }

    // Pytest: "47 passed, 3 failed"
    if let Some(counts) = parse_pytest_output(output) {
        return counts;
    }

    // Go: "ok  	package	0.123s" lines (count successes)
    if let Some(counts) = parse_go_test_output(output) {
        return counts;
    }

    (0, 0, 0)
}

/// Parse Rust test output: "test result: ok. N passed; N failed; N ignored"
fn parse_rust_test_output(output: &str) -> Option<(usize, usize, usize)> {
    for line in output.lines() {
        if line.contains("test result:") {
            let passed = extract_number_before(line, "passed").unwrap_or(0);
            let failed = extract_number_before(line, "failed").unwrap_or(0);
            let ignored = extract_number_before(line, "ignored").unwrap_or(0);
            return Some((passed, failed, ignored));
        }
    }
    None
}

/// Parse Jest/Vitest output: "Tests:  3 failed, 47 passed, 50 total"
fn parse_jest_output(output: &str) -> Option<(usize, usize, usize)> {
    for line in output.lines() {
        if line.contains("Tests:") && line.contains("total") {
            let passed = extract_number_before(line, "passed").unwrap_or(0);
            let failed = extract_number_before(line, "failed").unwrap_or(0);
            let skipped = extract_number_before(line, "skipped").unwrap_or(0);
            return Some((passed, failed, skipped));
        }
    }
    None
}

/// Parse pytest output: "47 passed, 3 failed"
fn parse_pytest_output(output: &str) -> Option<(usize, usize, usize)> {
    for line in output.lines() {
        if line.contains(" passed") && (line.starts_with('=') || line.contains("===")) {
            let passed = extract_number_before(line, "passed").unwrap_or(0);
            let failed = extract_number_before(line, "failed").unwrap_or(0);
            let skipped = extract_number_before(line, "skipped").unwrap_or(0);
            return Some((passed, failed, skipped));
        }
    }
    None
}

/// Parse Go test output: count "ok" and "FAIL" lines.
fn parse_go_test_output(output: &str) -> Option<(usize, usize, usize)> {
    let mut ok = 0;
    let mut fail = 0;
    let mut found = false;
    for line in output.lines() {
        if line.starts_with("ok") && line.contains('\t') {
            ok += 1;
            found = true;
        } else if line.starts_with("FAIL") && line.contains('\t') {
            fail += 1;
            found = true;
        }
    }
    if found { Some((ok, fail, 0)) } else { None }
}

/// Extract the number immediately before a keyword: "47 passed" → 47
fn extract_number_before(line: &str, keyword: &str) -> Option<usize> {
    let idx = line.find(keyword)?;
    let before = &line[..idx].trim_end();
    let num_str = before.rsplit(|c: char| !c.is_ascii_digit()).next()?;
    num_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust_output() {
        let output = "test result: ok. 47 passed; 2 failed; 3 ignored; 0 measured; 0 filtered out";
        let (p, f, i) = parse_rust_test_output(output).unwrap();
        assert_eq!((p, f, i), (47, 2, 3));
    }

    #[test]
    fn test_parse_jest_output() {
        let output = "Tests:  3 failed, 47 passed, 50 total";
        let (p, f, _) = parse_jest_output(output).unwrap();
        assert_eq!((p, f), (47, 3));
    }

    #[test]
    fn test_parse_pytest_output() {
        let output = "======= 47 passed, 3 failed in 12.3s =======";
        let (p, f, _) = parse_pytest_output(output).unwrap();
        assert_eq!((p, f), (47, 3));
    }

    #[test]
    fn test_parse_go_test_output() {
        let output = "ok  \tgithub.com/user/pkg\t0.123s\nFAIL\tgithub.com/user/bad\t0.456s\n";
        let (p, f, _) = parse_go_test_output(output).unwrap();
        assert_eq!((p, f), (1, 1));
    }

    #[test]
    fn test_extract_number_before() {
        assert_eq!(extract_number_before("47 passed", "passed"), Some(47));
        assert_eq!(extract_number_before("3 failed, 47 passed", "passed"), Some(47));
        assert_eq!(extract_number_before("no match here", "passed"), None);
    }

    #[test]
    fn test_test_result_summary() {
        let result = TestResult {
            command: "cargo test".into(),
            passed: 47,
            failed: 2,
            ignored: 3,
            total: 52,
            duration_secs: 12.3,
            success: false,
            raw_output: super::super::setup::CommandOutput {
                command: "cargo test".into(),
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 101,
                success: false,
                duration: std::time::Duration::from_secs(12),
            },
        };
        let s = result.summary();
        assert!(s.contains("47/52"));
        assert!(s.contains("2 failed"));
    }

    #[test]
    fn test_run_tests_echo() {
        let dir = std::env::temp_dir();
        let result = run_tests("echo 'test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured'", &dir, 10);
        assert!(result.success);
        assert_eq!(result.passed, 5);
        assert_eq!(result.total, 5);
    }
}
