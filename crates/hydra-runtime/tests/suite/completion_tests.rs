use hydra_runtime::completion::{ChangeType, ChangeSummary, CompletionSummary};

#[test]
fn test_new_summary() {
    let summary = CompletionSummary::new("Refactored auth module");
    assert_eq!(summary.headline, "Refactored auth module");
    assert!(summary.actions.is_empty());
    assert!(summary.changes.is_empty());
    assert!(summary.next_steps.is_empty());
    assert!(summary.warnings.is_empty());
}

#[test]
fn test_add_action() {
    let mut summary = CompletionSummary::new("Test");
    summary.add_action("Created new module");
    summary.add_action("Updated imports");
    assert_eq!(summary.actions.len(), 2);
    assert_eq!(summary.actions[0], "Created new module");
}

#[test]
fn test_add_change() {
    let mut summary = CompletionSummary::new("Test");
    summary.add_change(ChangeSummary::new("src/lib.rs", ChangeType::Modified, 10, 3));
    summary.add_change(ChangeSummary::new("src/new.rs", ChangeType::Created, 50, 0));
    assert_eq!(summary.changes.len(), 2);
    assert_eq!(summary.stats.files_affected, 2);
}

#[test]
fn test_total_lines() {
    let mut summary = CompletionSummary::new("Test");
    summary.add_change(ChangeSummary::new("a.rs", ChangeType::Modified, 10, 3));
    summary.add_change(ChangeSummary::new("b.rs", ChangeType::Modified, 5, 2));
    assert_eq!(summary.total_lines_added(), 15);
    assert_eq!(summary.total_lines_removed(), 5);
}

#[test]
fn test_format_cli_output() {
    let mut summary = CompletionSummary::new("Built project");
    summary.add_action("Compiled 10 files");
    summary.add_change(ChangeSummary::new("src/main.rs", ChangeType::Modified, 5, 2));
    summary.add_next_step("Run tests");
    summary.stats.duration_ms = 1234;
    summary.stats.tokens_used = 500;

    let cli = summary.format_cli();
    assert!(cli.contains("Done: Built project"));
    assert!(cli.contains("Compiled 10 files"));
    assert!(cli.contains("~ src/main.rs"));
    assert!(cli.contains("1234ms elapsed"));
    assert!(cli.contains("Run tests"));
}

#[test]
fn test_format_voice_output() {
    let mut summary = CompletionSummary::new("Updated config");
    summary.add_change(ChangeSummary::new("config.toml", ChangeType::Modified, 3, 1));
    summary.add_next_step("Restart the service");

    let voice = summary.format_voice();
    assert!(voice.starts_with("Done!"));
    assert!(voice.contains("1 change"));
    assert!(voice.contains("1 file"));
    assert!(voice.contains("Restart the service"));
}

#[test]
fn test_format_voice_plural() {
    let mut summary = CompletionSummary::new("Refactored");
    summary.add_change(ChangeSummary::new("a.rs", ChangeType::Modified, 1, 0));
    summary.add_change(ChangeSummary::new("b.rs", ChangeType::Created, 10, 0));

    let voice = summary.format_voice();
    assert!(voice.contains("2 changes"));
    assert!(voice.contains("2 files"));
}

#[test]
fn test_format_json_roundtrip() {
    let mut summary = CompletionSummary::new("Test roundtrip");
    summary.add_action("did something");
    summary.add_change(ChangeSummary::new("x.rs", ChangeType::Deleted, 0, 50));

    let json = summary.format_json();
    let parsed: CompletionSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.headline, "Test roundtrip");
    assert_eq!(parsed.changes[0].change_type, ChangeType::Deleted);
}

#[test]
fn test_change_type_symbols_in_cli() {
    let mut summary = CompletionSummary::new("Multi-change");
    summary.add_change(ChangeSummary::new("new.rs", ChangeType::Created, 10, 0));
    summary.add_change(ChangeSummary::new("old.rs", ChangeType::Deleted, 0, 20));
    summary.add_change(ChangeSummary::new("moved.rs", ChangeType::Renamed, 0, 0));

    let cli = summary.format_cli();
    assert!(cli.contains("+ new.rs"));
    assert!(cli.contains("- old.rs"));
    assert!(cli.contains("> moved.rs"));
}

#[test]
fn test_warnings_in_cli() {
    let mut summary = CompletionSummary::new("With warnings");
    summary.add_warning("Deprecated API used");

    let cli = summary.format_cli();
    assert!(cli.contains("WARNING: Deprecated API used"));
}
