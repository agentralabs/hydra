use std::process::Command;

fn hydra_cmd() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_hydra-cli"));
    cmd.env("NO_COLOR", "1"); // hint for future color disable support
    cmd
}

#[test]
fn test_help_output() {
    let output = hydra_cmd().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hydra"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("approve"));
    assert!(stdout.contains("sisters"));
    assert!(stdout.contains("skills"));
    assert!(stdout.contains("inspect"));
    assert!(stdout.contains("config"));
}

#[test]
fn test_version_output() {
    let output = hydra_cmd().arg("--version").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PRODUCTION RELEASE") || stdout.contains("Repository"));
}

#[test]
fn test_run_command_dry_run() {
    let output = hydra_cmd()
        .args(["run", "test intent", "--dry-run"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dry Run"));
    assert!(stdout.contains("test intent"));
    assert!(stdout.contains("dry run"));
}

#[test]
fn test_run_no_intent() {
    let output = hydra_cmd().arg("run").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No intent provided"));
}

#[test]
fn test_status_command() {
    let output = hydra_cmd().arg("status").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Without a running server, shows offline error
    assert!(stdout.contains("Server offline") || stdout.contains("Hydra Status"));
}

#[test]
fn test_status_with_run_id() {
    let output = hydra_cmd()
        .args(["status", "run_abc123"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Without a running server, shows unreachable error mentioning the run ID
    assert!(stdout.contains("run_abc123") || stdout.contains("Server unreachable"));
}

#[test]
fn test_config_show() {
    let output = hydra_cmd().args(["config", "show"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Configuration"));
    assert!(stdout.contains("auto_approve"));
    assert!(stdout.contains("max_tokens"));
}

#[test]
fn test_config_get() {
    let output = hydra_cmd()
        .args(["config", "get", "log_level"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("info"));
}

#[test]
fn test_config_set() {
    let output = hydra_cmd()
        .args(["config", "set", "log_level", "debug"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Set"));
    assert!(stdout.contains("log_level"));
}

#[test]
fn test_sisters_status() {
    let output = hydra_cmd()
        .args(["sisters", "status"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sisters"));
    assert!(stdout.contains("memory"));
    assert!(stdout.contains("vision"));
    assert!(stdout.contains("codebase"));
    assert!(stdout.contains("connected"));
}

#[test]
fn test_skills_list() {
    let output = hydra_cmd()
        .args(["skills", "list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Installed Skills"));
    assert!(stdout.contains("code-review"));
}

#[test]
fn test_completions_bash() {
    let output = hydra_cmd()
        .args(["completions", "bash"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("_hydra_completions"));
    assert!(stdout.contains("complete"));
}

#[test]
fn test_completions_zsh() {
    let output = hydra_cmd()
        .args(["completions", "zsh"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("_hydra"));
    assert!(stdout.contains("compdef"));
}

#[test]
fn test_completions_fish() {
    let output = hydra_cmd()
        .args(["completions", "fish"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("complete -c hydra"));
}

#[test]
fn test_intent_as_bare_argument() {
    // When an unknown command is given, treat it as an intent
    let output = hydra_cmd()
        .args(["deploy", "to", "staging"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hydra"));
    assert!(stdout.contains("deploy to staging"));
}
