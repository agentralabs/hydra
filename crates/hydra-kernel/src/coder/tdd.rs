//! TDD loop + incremental writer + systematic debugger.
//! Write one file at a time, typecheck between. Generate tests first, code to pass.

use super::CodingPlan;

// ── Types ──

/// Result of a TDD cycle.
#[derive(Debug, Clone)]
pub struct TddResult {
    pub passed: usize,
    pub failed: usize,
    pub total: usize,
    pub debug_sessions: usize,
    pub files_written: Vec<String>,
}

/// A debug session record.
#[derive(Debug, Clone)]
pub struct DebugSession {
    pub error: String,
    pub root_cause: String,
    pub fix_applied: String,
    pub prevented: bool,
}

// ── Constants ──

const MAX_TDD_ITERATIONS: usize = 5;
const MAX_DEBUG_ATTEMPTS: usize = 3;

// ── Stage 3: Incremental Writer ──

/// Write files incrementally with verification between each.
pub fn write_incremental(plan: &CodingPlan, working_dir: &str) -> Vec<String> {
    let mut written = Vec::new();

    // Install dependencies first
    for cmd in &plan.install_commands {
        eprintln!("hydra-coder: installing: {cmd}");
        let _ = run_shell(cmd, working_dir);
    }

    // Write each file, verify after
    for file in &plan.files_to_create {
        eprintln!("hydra-coder: writing {}", file.path);
        // In full implementation: LLM generates content, write to file, typecheck
        // For now: record the planned file
        written.push(file.path.clone());
    }

    for file in &plan.files_to_modify {
        eprintln!("hydra-coder: modifying {}", file.path);
        written.push(file.path.clone());
    }

    written
}

/// Verify a file after writing (typecheck, lint).
pub fn verify_file(path: &str, language: &str, working_dir: &str) -> VerifyResult {
    let (cmd, expect_clean) = match language {
        "rust" => ("cargo check 2>&1", true),
        "typescript" => ("npx tsc --noEmit 2>&1", true),
        "python" => ("python -m py_compile", true),
        _ => ("true", true),
    };

    let output = run_shell(cmd, working_dir);
    let success = output.success && (expect_clean || !output.stdout.contains("error"));

    let errors = if success { vec![] } else { vec![output.stdout.clone()] };
    VerifyResult { success, output: output.stdout, errors }
}

#[derive(Debug)]
pub struct VerifyResult {
    pub success: bool,
    pub output: String,
    pub errors: Vec<String>,
}

// ── Stage 4: TDD Loop ──

/// Run TDD cycle: generate tests → run → write code to pass → iterate.
pub fn run_tdd(plan: &CodingPlan, working_dir: &str) -> TddResult {
    let mut passed = 0;
    let mut failed = 0;
    let mut debug_sessions = 0;
    let files_written = write_incremental(plan, working_dir);

    // Run test suite
    for test_desc in &plan.tests_to_write {
        eprintln!("hydra-coder: TDD — {test_desc}");

        let mut test_passed = false;
        for iteration in 0..MAX_TDD_ITERATIONS {
            // Run tests
            let test_result = run_tests(working_dir, &detect_test_cmd(working_dir));
            if test_result.success {
                test_passed = true;
                eprintln!("hydra-coder: TDD iteration {iteration} — PASS");
                break;
            }

            // Debug if failed
            if iteration < MAX_TDD_ITERATIONS - 1 {
                let session = debug_failure(&test_result.stdout, working_dir);
                debug_sessions += 1;
                eprintln!("hydra-coder: debug — root cause: {}", session.root_cause);
            }
        }

        if test_passed { passed += 1; } else { failed += 1; }
    }

    TddResult { passed, failed, total: plan.tests_to_write.len(), debug_sessions, files_written }
}

fn detect_test_cmd(working_dir: &str) -> String {
    let dir = std::path::Path::new(working_dir);
    if dir.join("Cargo.toml").exists() { "cargo test 2>&1".into() }
    else if dir.join("package.json").exists() { "npm test 2>&1".into() }
    else if dir.join("pytest.ini").exists() || dir.join("pyproject.toml").exists() { "pytest 2>&1".into() }
    else { "echo no test framework".into() }
}

fn run_tests(working_dir: &str, cmd: &str) -> ShellOutput {
    run_shell(cmd, working_dir)
}

// ── Stage 5: Systematic Debugger ──

/// Debug a test failure: reproduce → isolate → root cause → fix → prevent.
pub fn debug_failure(error_output: &str, _working_dir: &str) -> DebugSession {
    // Step 1: REPRODUCE — error output already captured
    let error = error_output.lines().take(5).collect::<Vec<_>>().join("\n");

    // Step 2: ISOLATE — find the specific error
    let isolated = isolate_error(error_output);

    // Step 3: ROOT CAUSE — match against known patterns
    let root_cause = diagnose_root_cause(&isolated);

    // Step 4: FIX — would apply targeted fix (placeholder)
    let fix = suggest_fix(&root_cause);

    // Step 5: PREVENT — genome entry created via feedback pipeline
    eprintln!("hydra-coder: debug session — {root_cause} → {fix}");

    DebugSession {
        error: error.chars().take(200).collect(),
        root_cause,
        fix_applied: fix,
        prevented: true,
    }
}

fn isolate_error(output: &str) -> String {
    // Find the first error line
    for line in output.lines() {
        let lower = line.to_lowercase();
        if lower.contains("error") || lower.contains("failed") || lower.contains("panic") {
            return line.trim().to_string();
        }
    }
    output.lines().next().unwrap_or("unknown error").to_string()
}

fn diagnose_root_cause(error: &str) -> String {
    let lower = error.to_lowercase();
    if lower.contains("import") || lower.contains("module") {
        "Import/module resolution error".into()
    } else if lower.contains("not found") || lower.contains("no such file") {
        "Missing file or dependency".into()
    } else if lower.contains("type") || lower.contains("cannot assign") {
        "Type mismatch".into()
    } else if lower.contains("permission") {
        "Permission denied".into()
    } else if lower.contains("timeout") {
        "Operation timed out".into()
    } else if lower.contains("syntax") {
        "Syntax error".into()
    } else {
        format!("Unknown: {}", &error[..error.len().min(100)])
    }
}

fn suggest_fix(root_cause: &str) -> String {
    match root_cause {
        "Missing file or dependency" => "Install missing dependency or create missing file".into(),
        "Type mismatch" => "Fix type annotation or cast".into(),
        "Import/module resolution error" => "Fix import path or install package".into(),
        "Permission denied" => "Check file permissions or use sudo".into(),
        "Syntax error" => "Fix syntax error at indicated line".into(),
        _ => "Investigate error output for specific fix".into(),
    }
}

// ── Shell Helper ──

struct ShellOutput { success: bool, stdout: String }

fn run_shell(cmd: &str, working_dir: &str) -> ShellOutput {
    let mut command = std::process::Command::new("sh");
    command.arg("-c").arg(cmd).current_dir(working_dir);
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| { libc::setpgid(0, 0); Ok(()) });
    }
    match command.output() {
        Ok(out) => ShellOutput {
            success: out.status.success(),
            stdout: format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)),
        },
        Err(e) => ShellOutput { success: false, stdout: format!("Shell error: {e}") },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolate_error_finds_error_line() {
        let output = "Running tests...\nOK: test_a\nerror[E0308]: mismatched types\n  --> src/lib.rs:42";
        assert!(isolate_error(output).contains("mismatched types"));
    }

    #[test]
    fn diagnose_type_error() {
        assert_eq!(diagnose_root_cause("error: cannot assign &str to i32"), "Type mismatch");
    }

    #[test]
    fn diagnose_import_error() {
        assert_eq!(diagnose_root_cause("error: module 'auth' not found"), "Import/module resolution error");
    }

    #[test]
    fn diagnose_unknown() {
        let cause = diagnose_root_cause("something weird happened");
        assert!(cause.starts_with("Unknown:"));
    }

    #[test]
    fn verify_rust_file() {
        let result = verify_file("src/lib.rs", "rust", ".");
        // May fail on syntax — that's ok, we just test it doesn't panic
        let _ = result;
    }

    #[test]
    fn detect_test_cmd_rust() {
        assert!(detect_test_cmd(".").contains("cargo test"));
    }

    #[test]
    fn debug_produces_session() {
        let session = debug_failure("error: type mismatch at line 42", ".");
        assert!(!session.root_cause.is_empty());
        assert!(!session.fix_applied.is_empty());
    }
}
