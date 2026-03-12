//! Integration tests for hydra-native desktop app.

use hydra_native::app::{AppSection, AppViewModel};
use hydra_native::commands::hydra::HydraCommands;
use hydra_native::components::globe::{globe_params, globe_svg};
use hydra_native::components::input::validate_input;
use hydra_native::components::message::parse_message;
use hydra_native::components::phases::build_phase_dots;
use hydra_native::state::hydra::*;
use hydra_native::styles::STYLES;

#[test]
fn test_app_state_init() {
    let state = HydraState::with_defaults();
    let vm = AppViewModel::from_state(&state);
    assert_eq!(vm.section, AppSection::Chat);
    assert_eq!(vm.globe_state, GlobeState::Idle);
    assert!(!vm.connected);
    assert_eq!(vm.message_count, 0);
    assert_eq!(vm.total_tokens, 0);
    assert!(vm.error.is_none());
}

#[test]
fn test_message_add_and_view() {
    let mut state = HydraState::with_defaults();
    state.add_user_message("Hello Hydra");
    state.add_hydra_message("Hi! How can I help?", None, Some(50));

    let vm = AppViewModel::from_state(&state);
    assert_eq!(vm.message_count, 2);
    assert_eq!(vm.total_tokens, 50);

    assert_eq!(state.messages[0].role, MessageRole::User);
    assert_eq!(state.messages[1].role, MessageRole::Hydra);
}

#[test]
fn test_phase_transitions_full_cycle() {
    let mut state = HydraState::with_defaults();
    state.set_connected(true);

    // Start a run — goes through all 5 phases
    state.handle_run_started("run-1", "Sort a list");
    assert_eq!(state.globe_state, GlobeState::Processing);

    for &phase in CognitivePhase::ALL {
        state.handle_step_started("run-1", phase);
        assert_eq!(state.active_phase(), Some(phase));

        let dots = build_phase_dots(&state.current_run.as_ref().unwrap().phases);
        let running_dot = dots.iter().find(|d| d.phase == phase).unwrap();
        assert_eq!(running_dot.state, PhaseState::Running);
        assert_eq!(running_dot.css_class, "phase-running");

        state.handle_step_completed("run-1", phase, Some(100), Some(50));
        assert_eq!(state.active_phase(), None);
    }

    // Complete the run
    state.handle_run_completed("run-1", Some("Sorted!"), Some(500));
    assert_eq!(state.globe_state, GlobeState::Idle);

    let vm = AppViewModel::from_state(&state);
    assert_eq!(vm.status_text(), "Ready | 500 tokens used");

    // All phases should be completed
    let dots = build_phase_dots(&state.current_run.as_ref().unwrap().phases);
    assert!(dots.iter().all(|d| d.state == PhaseState::Completed));
}

#[test]
fn test_globe_all_states_render() {
    let states = [
        GlobeState::Idle,
        GlobeState::Listening,
        GlobeState::Processing,
        GlobeState::Speaking,
        GlobeState::Error,
        GlobeState::Approval,
    ];

    for &gs in &states {
        let params = globe_params(gs);
        assert_eq!(params.state, gs);

        let svg = globe_svg(&params, 64);
        assert!(
            svg.contains("svg"),
            "SVG for {:?} should contain svg tag",
            gs
        );
        assert!(
            svg.contains("circle"),
            "SVG for {:?} should contain circle",
            gs
        );
        assert!(
            svg.contains(params.fill),
            "SVG for {:?} should contain fill color",
            gs
        );

        // CSS class should exist in our stylesheet
        let class = gs.css_class();
        assert!(STYLES.contains(class), "Style for {} should exist", class);
    }
}

#[tokio::test]
async fn test_command_send_message() {
    let cmds = HydraCommands::with_defaults();
    let result = cmds.send_message("Write a sort function in Rust").await;
    assert!(result.success);
    let data = result.data.unwrap();
    assert!(!data.run_id.is_empty());
    assert_eq!(data.status, "started");

    // Verify run counter incremented
    let status = cmds.get_status().await.data.unwrap();
    assert_eq!(status.total_runs, 1);
}

#[tokio::test]
async fn test_command_kill() {
    let cmds = HydraCommands::with_defaults();

    // Valid kill levels
    for level in &["graceful", "immediate", "halt"] {
        let result = cmds.kill_run("run-1", level).await;
        assert!(result.success, "Kill with level '{}' should succeed", level);
    }

    // Invalid level
    let result = cmds.kill_run("run-1", "nuke").await;
    assert!(!result.success);

    // Empty run_id
    let result = cmds.kill_run("", "graceful").await;
    assert!(!result.success);
}

#[tokio::test]
async fn test_command_approve_deny() {
    let cmds = HydraCommands::with_defaults();

    let approve = cmds.approve("approval-1", true).await;
    assert!(approve.success);

    let deny = cmds.approve("approval-2", false).await;
    assert!(deny.success);

    // Empty ID
    let empty = cmds.approve("", true).await;
    assert!(!empty.success);
}

#[test]
fn test_style_generation() {
    // Verify CSS contains all required animation keyframes
    assert!(STYLES.contains("@keyframes globe-breathe"));
    assert!(STYLES.contains("@keyframes globe-pulse"));
    assert!(STYLES.contains("@keyframes globe-rotate"));
    assert!(STYLES.contains("@keyframes globe-shake"));
    assert!(STYLES.contains("@keyframes globe-glow"));
    assert!(STYLES.contains("@keyframes globe-ring-out"));
    assert!(STYLES.contains("@keyframes fade-in"));

    // Verify CSS variables
    assert!(STYLES.contains("--bg-primary"));
    assert!(STYLES.contains("--accent"));
    assert!(STYLES.contains("--error"));

    // HTML wrapper works
    let html = hydra_native::styles::html_wrapper("<div>Test</div>");
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<div>Test</div>"));
}

#[test]
fn test_markdown_render() {
    use hydra_native::components::message::MessageSegment;

    // Complex message with multiple elements
    let segments = parse_message("Use `cargo test` to run **all tests** in the workspace");
    assert!(segments.len() >= 4);
    assert!(segments
        .iter()
        .any(|s| matches!(s, MessageSegment::InlineCode(c) if c == "cargo test")));
    assert!(segments
        .iter()
        .any(|s| matches!(s, MessageSegment::Bold(b) if b == "all tests")));
}

#[test]
fn test_input_validation() {
    // Valid
    let v = validate_input("hello", 1000);
    assert!(v.valid);
    assert_eq!(v.trimmed, "hello");

    // Empty
    let v = validate_input("   ", 1000);
    assert!(!v.valid);

    // Too long
    let long = "a".repeat(101);
    let v = validate_input(&long, 100);
    assert!(!v.valid);
    assert!(v.error.unwrap().contains("too long"));

    // Whitespace trimmed
    let v = validate_input("  hello world  ", 1000);
    assert!(v.valid);
    assert_eq!(v.trimmed, "hello world");
}

#[test]
fn test_settings_persistence() {
    use hydra_native::components::settings::{apply_field, config_to_fields, SettingsValue};
    use hydra_native::state::hydra::Theme;

    let mut config = AppConfig::default();
    assert_eq!(config.theme, Theme::Dark);

    let fields = config_to_fields(&config);
    assert_eq!(fields.len(), 6);

    // Apply changes
    apply_field(
        &mut config,
        "theme",
        &SettingsValue::Choice {
            selected: "light".into(),
            options: vec![],
        },
    );
    assert_eq!(config.theme, Theme::Light);

    apply_field(&mut config, "voice_enabled", &SettingsValue::Bool(true));
    assert!(config.voice_enabled);

    apply_field(
        &mut config,
        "server_url",
        &SettingsValue::Text("http://custom:8080".into()),
    );
    assert_eq!(config.server_url, "http://custom:8080");
}

#[tokio::test]
async fn test_integration_e2e() {
    // Full end-to-end: send message → phases progress → response
    let mut state = HydraState::with_defaults();
    state.set_connected(true);

    let cmds = HydraCommands::with_defaults();

    // 1. Send message
    state.add_user_message("Write a hello world in Rust");
    let result = cmds.send_message("Write a hello world in Rust").await;
    assert!(result.success);
    let run_id = result.data.unwrap().run_id;

    // 2. Simulate SSE events
    state.handle_run_started(&run_id, "Write a hello world in Rust");
    let vm = AppViewModel::from_state(&state);
    assert_eq!(vm.globe_state, GlobeState::Processing);

    // 3. Progress through phases
    for &phase in CognitivePhase::ALL {
        state.handle_step_started(&run_id, phase);
        state.handle_step_completed(&run_id, phase, Some(80), Some(30));
    }

    // 4. Complete with response
    let response = r#"Here's a hello world in Rust:
```rust
fn main() {
    println!("Hello, world!");
}
```"#;
    state.handle_run_completed(&run_id, Some(response), Some(400));

    // 5. Verify final state
    assert_eq!(state.globe_state, GlobeState::Idle);
    assert_eq!(state.messages.len(), 2); // user + hydra response
    assert_eq!(state.messages[1].role, MessageRole::Hydra);
    assert!(state.messages[1].content.contains("Hello, world!"));

    let vm = AppViewModel::from_state(&state);
    assert_eq!(vm.total_tokens, 400);
    assert_eq!(vm.status_text(), "Ready | 400 tokens used");

    // 6. Verify events were logged
    let events = state.recent_events();
    assert!(events.contains(&"run_started"));
    assert!(events.contains(&"run_completed"));
}
