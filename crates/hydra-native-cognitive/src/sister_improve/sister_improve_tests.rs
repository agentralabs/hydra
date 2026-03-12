//! Tests for sister improvement engine.

use super::*;
use std::path::PathBuf;

// ── Analyzer tests ──

#[test]
fn test_analyze_sister_project() {
    // Use our own workspace as a test subject
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let analysis = analyzer::analyze_sister(&root).unwrap();
    assert_eq!(analysis.language, analyzer::SisterLanguage::Rust);
    assert!(!analysis.source_files.is_empty());
    assert!(!analysis.project_name.is_empty());
}

#[test]
fn test_analyze_nonexistent_path() {
    let result = analyzer::analyze_sister(Path::new("/nonexistent/path"));
    assert!(result.is_err());
}

#[test]
fn test_identify_limitation_auto() {
    let analysis = analyzer::SisterAnalysis {
        language: analyzer::SisterLanguage::Rust,
        project_name: "test".into(),
        source_files: vec![],
        test_files: vec![],
        has_ci: false,
        test_command: "cargo test".into(),
        build_command: "cargo check".into(),
        doc_files: vec![],
    };
    let baseline = verifier::TestResults::empty();
    let lim = analyzer::identify_limitation(&analysis, "auto-detect", &baseline);
    assert!(lim.contains("test coverage"));
}

#[test]
fn test_identify_limitation_specific_goal() {
    let analysis = analyzer::SisterAnalysis {
        language: analyzer::SisterLanguage::Rust,
        project_name: "test".into(),
        source_files: vec![PathBuf::from("src/lib.rs")],
        test_files: vec![PathBuf::from("tests/test.rs")],
        has_ci: true,
        test_command: "cargo test".into(),
        build_command: "cargo check".into(),
        doc_files: vec![PathBuf::from("README.md")],
    };
    let baseline = verifier::TestResults::empty();
    let lim = analyzer::identify_limitation(&analysis, "add retry logic", &baseline);
    assert_eq!(lim, "add retry logic");
}

// ── Test results parsing ──

#[test]
fn test_parse_cargo_output() {
    let output = "test result: ok. 15 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out";
    let results = verifier::TestResults::parse_cargo(output);
    assert_eq!(results.pass_count, 15);
    assert_eq!(results.fail_count, 2);
    assert_eq!(results.skip_count, 1);
    assert_eq!(results.total, 18);
}

#[test]
fn test_parse_cargo_multiple_suites() {
    let output = "\
test result: ok. 10 passed; 0 failed; 0 ignored
test result: ok. 5 passed; 1 failed; 0 ignored";
    let results = verifier::TestResults::parse_cargo(output);
    assert_eq!(results.pass_count, 15);
    assert_eq!(results.fail_count, 1);
}

#[test]
fn test_parse_pytest_output() {
    let output = "=== 5 passed, 2 failed, 1 skipped ===";
    let results = verifier::TestResults::parse_pytest(output);
    assert_eq!(results.pass_count, 5);
    assert_eq!(results.fail_count, 2);
    assert_eq!(results.skip_count, 1);
}

#[test]
fn test_parse_go_output() {
    let output = "ok      mypackage       0.5s\nFAIL    mypackage/sub   0.2s";
    let results = verifier::TestResults::parse_go(output);
    assert_eq!(results.pass_count, 1);
    assert_eq!(results.fail_count, 1);
}

// ── Verification ──

#[test]
fn test_verification_improved() {
    let baseline = verifier::TestResults { pass_count: 10, fail_count: 0, ..verifier::TestResults::empty() };
    let after = verifier::TestResults { pass_count: 12, fail_count: 0, ..verifier::TestResults::empty() };
    assert_eq!(verifier::verify(&baseline, &after), verifier::VerificationResult::Improved);
}

#[test]
fn test_verification_regressed() {
    let baseline = verifier::TestResults { pass_count: 10, fail_count: 0, ..verifier::TestResults::empty() };
    let after = verifier::TestResults { pass_count: 10, fail_count: 2, ..verifier::TestResults::empty() };
    assert_eq!(verifier::verify(&baseline, &after), verifier::VerificationResult::Regressed);
}

#[test]
fn test_verification_neutral() {
    let baseline = verifier::TestResults { pass_count: 10, fail_count: 0, ..verifier::TestResults::empty() };
    let after = verifier::TestResults { pass_count: 10, fail_count: 0, ..verifier::TestResults::empty() };
    assert_eq!(verifier::verify(&baseline, &after), verifier::VerificationResult::Neutral);
}

#[test]
fn test_no_regressions_check() {
    let baseline = verifier::TestResults { pass_count: 10, fail_count: 0, ..verifier::TestResults::empty() };
    let after = verifier::TestResults { pass_count: 8, fail_count: 0, ..verifier::TestResults::empty() };
    // Lost 2 passing tests = regression
    assert_eq!(verifier::verify(&baseline, &after), verifier::VerificationResult::Regressed);
}

// ── Patch generation ──

#[test]
fn test_generate_test_patch() {
    let request = patch_generator::PatchRequest {
        sister_path: PathBuf::from("/tmp/test-sister"),
        limitation: "Add test coverage".into(),
        goal: "add tests".into(),
        analysis: analyzer::SisterAnalysis {
            language: analyzer::SisterLanguage::Rust,
            project_name: "test-sister".into(),
            source_files: vec![],
            test_files: vec![],
            has_ci: false,
            test_command: "cargo test".into(),
            build_command: "cargo check".into(),
            doc_files: vec![],
        },
    };
    let patch = patch_generator::generate_patch(&request).unwrap();
    assert!(patch.description.contains("test"));
    assert!(!patch.changes.is_empty());
}

#[test]
fn test_generate_ci_patch() {
    let request = patch_generator::PatchRequest {
        sister_path: PathBuf::from("/tmp/test-sister"),
        limitation: "Add CI configuration".into(),
        goal: "add ci".into(),
        analysis: analyzer::SisterAnalysis {
            language: analyzer::SisterLanguage::TypeScript,
            project_name: "test-sister".into(),
            source_files: vec![],
            test_files: vec![],
            has_ci: false,
            test_command: "npm test".into(),
            build_command: "npm run build".into(),
            doc_files: vec![],
        },
    };
    let patch = patch_generator::generate_patch(&request).unwrap();
    assert!(patch.description.contains("CI"));
    assert!(patch.changes[0].content.contains("npm test"));
}

// ── Improvement report ──

#[test]
fn test_improvement_report_success() {
    let baseline = verifier::TestResults { pass_count: 10, fail_count: 0, ..verifier::TestResults::empty() };
    let after = verifier::TestResults { pass_count: 12, fail_count: 0, ..verifier::TestResults::empty() };
    let patch = patch_generator::ImprovementPatch {
        description: "test".into(),
        target_files: vec![],
        changes: vec![],
    };
    let report = ImprovementReport::success(baseline, after, patch);
    assert_eq!(report.status, ImprovementStatus::Success);
    assert!(report.summary().contains("12 tests passing"));
}

#[test]
fn test_improvement_report_reverted() {
    let baseline = verifier::TestResults { pass_count: 10, fail_count: 0, ..verifier::TestResults::empty() };
    let after = verifier::TestResults { pass_count: 8, fail_count: 2, ..verifier::TestResults::empty() };
    let patch = patch_generator::ImprovementPatch {
        description: "test".into(),
        target_files: vec![],
        changes: vec![],
    };
    let report = ImprovementReport::reverted(baseline, after, patch);
    assert_eq!(report.status, ImprovementStatus::Reverted);
    assert!(report.summary().contains("regressions"));
}

// ── Path extraction ──

#[test]
fn test_extract_sister_path() {
    // Can't test with real paths, but test the parse logic
    let text = "/improve-sister /tmp add retry";
    let path = extract_sister_path(text);
    // /tmp exists on macOS
    assert!(path.is_some());
    assert_eq!(path.unwrap(), PathBuf::from("/tmp"));
}

#[test]
fn test_extract_goal() {
    let text = "/improve-sister ../agentic-memory add retry logic to MCP transport";
    let goal = extract_goal(text);
    assert!(goal.contains("retry logic"));
}

#[test]
fn test_extract_goal_auto() {
    let text = "/improve-sister ../agentic-memory --auto";
    let goal = extract_goal(text);
    assert!(goal.contains("auto"));
}
