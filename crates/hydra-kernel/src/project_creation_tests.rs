//! Tests for project creation mode.

use super::*;

#[test]
fn test_detect_new_project_echo() {
    let spec = r#"# HYDRA SISTER FACTORY — VALIDATION TEST
AgenticEcho (.aecho)
Purpose: Echoes back messages
MCP Tools (5):
  echo_send     — Store a message
  echo_query    — Query stored messages
  echo_history  — List recent messages
  echo_stats    — Word count stats
  echo_clear    — Clear all messages
File format: .aecho (SQLite-backed)
Build the simplest possible sister. Not a real product — a validation target."#;
    let config = detect_new_project(spec).unwrap();
    assert_eq!(config.key, "echo");
    assert_eq!(config.name, "AgenticEcho");
    assert_eq!(config.tools.len(), 5);
    assert!(config.tools.contains(&"echo_send".to_string()));
}

#[test]
fn test_detect_existing_sister_returns_none() {
    let spec = "Improve AgenticMemory with better caching";
    assert!(detect_new_project(spec).is_none());
}

#[test]
fn test_detect_non_project_returns_none() {
    let spec = "Add a /version command to the TUI";
    assert!(detect_new_project(spec).is_none());
}

#[test]
fn test_capitalize() {
    assert_eq!(capitalize("echo"), "Echo");
    assert_eq!(capitalize("DATA"), "Data");
    assert_eq!(capitalize(""), "");
}

#[test]
fn test_scaffold_creates_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let config = ProjectConfig {
        key: "test".into(),
        name: "AgenticTest".into(),
        file_ext: ".atest".into(),
        cli_binary: "atest".into(),
        tools: vec!["test_send".into(), "test_query".into()],
        description: "Test sister".into(),
        target_dir: tmp.path().join("agentic-test"),
    };
    let dir = scaffold_workspace(&config).unwrap();
    assert!(dir.join("Cargo.toml").exists());
    assert!(dir.join("crates/agentic-test/src/lib.rs").exists());
    assert!(dir.join("crates/agentic-test-mcp/src/main.rs").exists());
    assert!(dir.join("crates/agentic-test-mcp/src/tools/registry.rs").exists());
    assert!(dir.join("crates/agentic-test-cli/src/main.rs").exists());
    assert!(dir.join("crates/agentic-test-ffi/src/lib.rs").exists());
}

#[test]
fn test_detect_new_project_data() {
    let spec = "Build a new sister AgenticData (.adat)\n  data_insert — Insert record\n  data_query — Query records";
    let config = detect_new_project(spec).unwrap();
    assert_eq!(config.key, "data");
    assert_eq!(config.name, "AgenticData");
    assert_eq!(config.tools.len(), 2);
}

#[test]
fn test_apply_project_patch_overwrites() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "fn old() {}").unwrap();

    let patch = crate::self_modify::Patch {
        target_file: "src/lib.rs".into(),
        gap: crate::self_modify::SpecGap {
            description: "test".into(),
            target_file: "src/lib.rs".into(),
            gap_type: crate::self_modify::GapType::MissingFunction,
            priority: 1,
        },
        diff_content: "fn new() {}\nfn also_new() {}".into(),
        description: "replace".into(),
        touches_critical: false,
    };

    apply_project_patch(dir, &patch).unwrap();
    let content = std::fs::read_to_string(dir.join("src/lib.rs")).unwrap();
    assert_eq!(content, "fn new() {}\nfn also_new() {}");
    assert!(!content.contains("old")); // old content gone
}

#[test]
#[ignore] // Downloads crates from registry — run manually
fn test_scaffold_compiles() {
    let tmp = tempfile::tempdir().unwrap();
    let config = ProjectConfig {
        key: "echo".into(),
        name: "AgenticEcho".into(),
        file_ext: ".aecho".into(),
        cli_binary: "aecho".into(),
        tools: vec![
            "echo_send".into(), "echo_query".into(),
            "echo_history".into(), "echo_stats".into(),
            "echo_clear".into(),
        ],
        description: "AgenticEcho validation sister".into(),
        target_dir: tmp.path().join("agentic-echo"),
    };
    let dir = scaffold_workspace(&config).unwrap();
    run_cargo_check_project(&dir).expect("Scaffolded project must compile");
    run_cargo_test_project(&dir).expect("Scaffolded project tests must pass");
}
