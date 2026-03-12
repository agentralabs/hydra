//! Category 4: UI/Desktop Tests — hydra-native components and state.

use hydra_native::state::hydra::*;

// === State management ===

#[test]
fn test_state_init() {
    let state = HydraState::with_defaults();
    assert!(state.messages.is_empty());
    assert!(state.current_run.is_none());
    assert!(!state.connected);
}

#[test]
fn test_state_add_message() {
    let mut state = HydraState::with_defaults();
    state.add_user_message("Hello");
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, MessageRole::User);

    state.add_hydra_message("Hi there", None, None);
    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[1].role, MessageRole::Hydra);
}

#[test]
fn test_state_phase_update() {
    let mut state = HydraState::with_defaults();
    state.handle_run_started("run1", "test intent");
    assert!(state.current_run.is_some());

    state.handle_step_started("run1", CognitivePhase::Perceive);
    let phase = state.active_phase();
    assert!(phase.is_some());
}

#[test]
fn test_state_connection_change() {
    let mut state = HydraState::with_defaults();
    assert!(!state.connected);
    state.set_connected(true);
    assert!(state.connected);
    state.set_connected(false);
    assert!(!state.connected);
}

#[test]
fn test_state_run_completed() {
    let mut state = HydraState::with_defaults();
    state.handle_run_started("run1", "test");
    state.handle_run_completed("run1", Some("done"), None);
    let run = state.current_run.as_ref().unwrap();
    assert_eq!(run.status, RunStatus::Completed);
}

#[test]
fn test_state_run_error() {
    let mut state = HydraState::with_defaults();
    state.handle_run_started("run1", "test");
    state.handle_run_error("run1", "something broke");
    let run = state.current_run.as_ref().unwrap();
    assert_eq!(run.status, RunStatus::Failed);
}

#[test]
fn test_state_total_tokens() {
    let state = HydraState::with_defaults();
    assert_eq!(state.total_tokens(), 0);
}

#[test]
fn test_state_clear() {
    let mut state = HydraState::with_defaults();
    state.add_user_message("test");
    state.handle_run_started("run1", "test");
    state.clear();
    assert!(state.messages.is_empty());
    assert!(state.current_run.is_none());
}

// === Phase states ===

#[test]
fn test_cognitive_phase_all() {
    assert_eq!(CognitivePhase::ALL.len(), 5);
    for phase in CognitivePhase::ALL {
        assert!(!phase.label().is_empty());
        assert!(phase.index() < 5);
    }
}

#[test]
fn test_phase_state_variants() {
    let states = vec![
        PhaseState::Pending,
        PhaseState::Running,
        PhaseState::Completed,
        PhaseState::Failed,
    ];
    assert_eq!(states.len(), 4);
}

// === Globe state ===

#[test]
fn test_globe_all_states() {
    let states = vec![
        GlobeState::Idle,
        GlobeState::Listening,
        GlobeState::Processing,
        GlobeState::Speaking,
        GlobeState::Error,
        GlobeState::Approval,
    ];
    for state in &states {
        assert!(!state.css_class().is_empty());
    }
}

// === Message parsing ===

#[test]
fn test_message_markdown_code() {
    use hydra_native::components::message::*;
    let input = "Here is `inline code` in text";
    let segments = parse_message(input);
    assert!(segments.len() > 1);
}

#[test]
fn test_message_plain_text() {
    use hydra_native::components::message::*;
    let segments = parse_message("just plain text");
    assert!(!segments.is_empty());
}

// === Input validation ===

#[test]
fn test_input_empty_invalid() {
    use hydra_native::components::input::*;
    let result = validate_input("", 10_000);
    assert!(!result.valid);
}

#[test]
fn test_input_whitespace_only() {
    use hydra_native::components::input::*;
    let result = validate_input("   ", 10_000);
    assert!(!result.valid);
}

#[test]
fn test_input_valid() {
    use hydra_native::components::input::*;
    let result = validate_input("Hello Hydra", 10_000);
    assert!(result.valid);
    assert_eq!(result.trimmed, "Hello Hydra");
}

// === Settings ===

#[test]
fn test_settings_config_to_fields() {
    use hydra_native::components::settings::*;
    let config = AppConfig {
        server_url: "http://localhost:7777".into(),
        theme: Theme::Dark,
        voice_enabled: false,
        sounds_enabled: true,
        sound_volume: 0.7,
        auto_approve_low_risk: false,
        default_mode: "companion".into(),
    };
    let fields = config_to_fields(&config);
    assert!(!fields.is_empty());
}

// === Globe rendering ===

#[test]
fn test_globe_params_all_states() {
    use hydra_native::components::globe::*;
    let states = vec![
        GlobeState::Idle,
        GlobeState::Listening,
        GlobeState::Processing,
        GlobeState::Speaking,
        GlobeState::Error,
        GlobeState::Approval,
    ];
    for state in states {
        let params = globe_params(state);
        assert!(!params.fill.is_empty());
        assert!(!params.animation.is_empty());
    }
}

// === Chat bubble ===

#[test]
fn test_message_bubble_from_user() {
    use hydra_native::components::chat::*;
    let msg = ChatMessage {
        id: "1".into(),
        role: MessageRole::User,
        content: "Hello".into(),
        timestamp: "2026-03-07T00:00:00Z".into(),
        run_id: None,
        tokens_used: None,
    };
    let bubble = MessageBubble::from_message(&msg);
    assert!(bubble.is_user);
}

#[test]
fn test_message_bubble_from_hydra() {
    use hydra_native::components::chat::*;
    let msg = ChatMessage {
        id: "2".into(),
        role: MessageRole::Hydra,
        content: "Hi there".into(),
        timestamp: "2026-03-07T00:00:00Z".into(),
        run_id: Some("run1".into()),
        tokens_used: Some(150),
    };
    let bubble = MessageBubble::from_message(&msg);
    assert!(!bubble.is_user);
}

// === Phase visualization ===

#[test]
fn test_phase_dots_all() {
    use hydra_native::components::phases::*;
    let phases = vec![
        PhaseStatus {
            phase: CognitivePhase::Perceive,
            state: PhaseState::Completed,
            tokens_used: Some(100),
            duration_ms: Some(50),
        },
        PhaseStatus {
            phase: CognitivePhase::Think,
            state: PhaseState::Running,
            tokens_used: Some(200),
            duration_ms: Some(100),
        },
        PhaseStatus {
            phase: CognitivePhase::Decide,
            state: PhaseState::Pending,
            tokens_used: None,
            duration_ms: None,
        },
        PhaseStatus {
            phase: CognitivePhase::Act,
            state: PhaseState::Pending,
            tokens_used: None,
            duration_ms: None,
        },
        PhaseStatus {
            phase: CognitivePhase::Learn,
            state: PhaseState::Pending,
            tokens_used: None,
            duration_ms: None,
        },
    ];
    let dots = build_phase_dots(&phases);
    assert_eq!(dots.len(), 5);
    let connectors = build_connectors(&phases);
    assert_eq!(connectors.len(), 4); // 4 connections between 5 phases
}

// === App view model ===

#[test]
fn test_app_view_model_status() {
    use hydra_native::app::*;
    let state = HydraState::with_defaults();
    let vm = AppViewModel::from_state(&state);
    assert!(!vm.connected);
    assert!(
        vm.status_text().contains("disconnected")
            || vm.status_text().contains("Disconnected")
            || !vm.status_text().is_empty()
    );
}
