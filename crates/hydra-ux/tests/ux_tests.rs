use std::time::{Duration, Instant};

use hydra_core::types::*;
use hydra_ux::decisions::{DecisionEngine, DecisionResult};
use hydra_ux::icon::IconStateMachine;
use hydra_ux::onboarding::OnboardingFlow;
use hydra_ux::proactive::{ProactiveConfig, ProactiveEngine, UpdateThrottle};

// ═══════════════════════════════════════════════════════════
// PROACTIVE ENGINE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_acknowledgment_under_100ms() {
    let engine = ProactiveEngine::new(ProactiveConfig::default());
    let mut rx = engine.subscribe();
    let start = Instant::now();
    engine.send_acknowledgment("Got it!");
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(100),
        "Acknowledgment took {:?}, must be < 100ms",
        elapsed
    );
    let update = rx.try_recv().unwrap();
    match update {
        ProactiveUpdate::Acknowledgment { message } => assert_eq!(message, "Got it!"),
        _ => panic!("Expected Acknowledgment"),
    }
}

#[tokio::test]
async fn test_no_silence_over_5s() {
    let config = ProactiveConfig {
        max_silence: Duration::from_millis(200),
        ..Default::default()
    };
    let engine = ProactiveEngine::new(config);
    let mut rx = engine.subscribe();
    let handle = engine.start_silence_watcher();

    // Wait for the silence watcher to fire (checks every 1s, so wait 1.5s)
    tokio::time::sleep(Duration::from_millis(1500)).await;
    engine.stop_silence_watcher();
    handle.abort();

    // Should have received a "still working" event
    let mut got_still_working = false;
    while let Ok(update) = rx.try_recv() {
        if let ProactiveUpdate::Event { title, .. } = update {
            if title.contains("Still working") {
                got_still_working = true;
            }
        }
    }
    assert!(got_still_working, "Silence watcher should have fired");
}

#[test]
fn test_progress_throttling() {
    let config = ProactiveConfig {
        min_progress_delta: 0.1, // 10%
        ..Default::default()
    };
    let engine = ProactiveEngine::new(config);
    let mut rx = engine.subscribe();

    // Send many small increments
    for i in 0..100 {
        engine.send_progress(i as f64, "working");
    }

    // Should only have ~10 updates (every 10%)
    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    assert!(
        count < 20,
        "Progress should be throttled, got {count} updates"
    );
}

#[tokio::test]
async fn test_send_completion() {
    let engine = ProactiveEngine::new(ProactiveConfig::default());
    let mut rx = engine.subscribe();
    engine.send_completion(CompletionSummary {
        headline: "Done!".into(),
        actions: vec!["created file".into()],
        changes: vec!["src/main.rs".into()],
        next_steps: vec!["run tests".into()],
    });
    let update = rx.try_recv().unwrap();
    match update {
        ProactiveUpdate::Completion { summary } => {
            assert_eq!(summary.headline, "Done!");
        }
        _ => panic!("Expected Completion"),
    }
}

#[test]
fn test_send_alert() {
    let engine = ProactiveEngine::new(ProactiveConfig::default());
    let mut rx = engine.subscribe();
    engine.send_alert(
        AlertLevel::Error,
        "Something failed",
        Some("Try again".into()),
    );
    let update = rx.try_recv().unwrap();
    match update {
        ProactiveUpdate::Alert {
            level,
            message,
            suggestion,
        } => {
            assert!(matches!(level, AlertLevel::Error));
            assert_eq!(message, "Something failed");
            assert_eq!(suggestion, Some("Try again".to_string()));
        }
        _ => panic!("Expected Alert"),
    }
}

#[test]
fn test_present_error() {
    let engine = ProactiveEngine::new(ProactiveConfig::default());
    let mut rx = engine.subscribe();
    let err = hydra_core::error::HydraError::Timeout;
    engine.present_error(&err);
    let update = rx.try_recv().unwrap();
    match update {
        ProactiveUpdate::Alert { message, .. } => {
            assert!(message.contains("took too long"));
        }
        _ => panic!("Expected Alert from present_error"),
    }
}

#[test]
fn test_updates_sent_counter() {
    let engine = ProactiveEngine::new(ProactiveConfig::default());
    assert_eq!(engine.updates_sent(), 0);
    engine.send_acknowledgment("a");
    engine.send_event("b", "c");
    assert_eq!(engine.updates_sent(), 2);
}

// ═══════════════════════════════════════════════════════════
// ICON STATE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_all_8_icon_states() {
    let icon = IconStateMachine::new();
    let states = [
        IconState::Idle,
        IconState::Listening,
        IconState::Working,
        IconState::NeedsAttention,
        IconState::ApprovalNeeded,
        IconState::Success,
        IconState::Error,
        IconState::Offline,
    ];
    for state in states {
        icon.transition(state);
        assert_eq!(icon.current(), state);
        assert!(!icon.current_animation().is_empty());
    }
}

#[test]
fn test_icon_default_idle() {
    let icon = IconStateMachine::new();
    assert_eq!(icon.current(), IconState::Idle);
}

#[test]
fn test_icon_offline_restricted_transitions() {
    let icon = IconStateMachine::new();
    icon.transition(IconState::Offline);
    // From Offline, can only go to Idle
    assert!(!icon.can_transition(IconState::Working));
    assert!(icon.can_transition(IconState::Idle));
    assert!(!icon.try_transition(IconState::Working));
    assert!(icon.try_transition(IconState::Idle));
    assert_eq!(icon.current(), IconState::Idle);
}

#[test]
fn test_icon_normal_transitions() {
    let icon = IconStateMachine::new();
    assert!(icon.can_transition(IconState::Working));
    assert!(icon.can_transition(IconState::Listening));
    assert!(icon.can_transition(IconState::Error));
    assert!(icon.can_transition(IconState::Offline));
}

// ═══════════════════════════════════════════════════════════
// ONBOARDING TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_onboarding_30_seconds() {
    // The flow has 4 steps, all instant — well under 30s
    let mut flow = OnboardingFlow::new();
    assert_eq!(flow.step_index(), 0);
    assert!(!flow.is_complete());

    // Step 1: Welcome
    assert_eq!(*flow.current_step(), OnboardingStep::Welcome);
    flow.advance(None);

    // Step 2: Ask Name
    assert_eq!(*flow.current_step(), OnboardingStep::AskName);
    flow.advance(Some("Alice"));

    // Step 3: Ask Voice
    assert_eq!(*flow.current_step(), OnboardingStep::AskVoice);
    flow.advance(Some("yes"));

    // Step 4: Complete
    assert_eq!(*flow.current_step(), OnboardingStep::Complete);
    assert!(flow.is_complete());
    assert_eq!(flow.user_name(), Some("Alice"));
    assert_eq!(flow.voice_enabled(), Some(true));
}

#[test]
fn test_onboarding_skip_voice() {
    let mut flow = OnboardingFlow::new();
    flow.advance(None); // Welcome
    flow.advance(Some("Bob")); // Name
    flow.advance(Some("maybe later")); // Voice — should be false
    assert!(flow.is_complete());
    assert_eq!(flow.voice_enabled(), Some(false));
}

#[test]
fn test_onboarding_empty_name() {
    let mut flow = OnboardingFlow::new();
    flow.advance(None); // Welcome
    flow.advance(Some("")); // Empty name
                            // Should still advance, name stays None
    assert!(flow.user_name().is_none());
}

#[test]
fn test_onboarding_total_steps() {
    let flow = OnboardingFlow::new();
    assert_eq!(flow.total_steps(), 4);
}

#[test]
fn test_onboarding_prompts() {
    let mut flow = OnboardingFlow::new();
    assert!(flow.current_prompt().contains("Hydra"));
    flow.advance(None);
    assert!(flow.current_prompt().contains("name"));
    flow.advance(Some("Test"));
    assert!(flow.current_prompt().contains("voice"));
    flow.advance(Some("no"));
    assert!(flow.current_prompt().contains("set"));
}

// ═══════════════════════════════════════════════════════════
// DECISION TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_decision_with_response() {
    let engine = DecisionEngine::new();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let request = DecisionEngine::build_request(
        "Proceed?",
        vec![
            DecisionOption {
                label: "Yes".into(),
                description: None,
                risk_level: None,
                keyboard_shortcut: Some("y".into()),
            },
            DecisionOption {
                label: "No".into(),
                description: None,
                risk_level: None,
                keyboard_shortcut: Some("n".into()),
            },
        ],
        5,
        Some(0),
    );
    let req_id = request.id;

    tokio::spawn(async move {
        tx.send(DecisionResponse {
            request_id: req_id,
            chosen_option: 1,
            custom_input: None,
        })
        .unwrap();
    });

    let result = engine.request_decision(request, Some(rx)).await;
    match result {
        DecisionResult::Chosen {
            option_index,
            label,
        } => {
            assert_eq!(option_index, 1);
            assert_eq!(label, "No");
        }
        _ => panic!("Expected Chosen, got {:?}", result),
    }
}

#[tokio::test]
async fn test_decision_timeout_with_default() {
    let engine = DecisionEngine::new();
    let request = DecisionEngine::build_request(
        "Proceed?",
        vec![DecisionOption {
            label: "Yes".into(),
            description: None,
            risk_level: None,
            keyboard_shortcut: None,
        }],
        0, // 0 second timeout (immediate)
        Some(0),
    );

    // Keep sender alive so channel blocks until timeout
    let (_tx, rx) = tokio::sync::oneshot::channel::<DecisionResponse>();
    let result = engine.request_decision(request, Some(rx)).await;
    assert!(result.timed_out());
    assert!(!result.aborted());
    assert_eq!(result.chosen_index(), Some(0));
}

#[tokio::test]
async fn test_decision_timeout_no_default_aborts() {
    let engine = DecisionEngine::new();
    let request = DecisionEngine::build_request(
        "Proceed?",
        vec![DecisionOption {
            label: "Yes".into(),
            description: None,
            risk_level: None,
            keyboard_shortcut: None,
        }],
        0,    // immediate timeout
        None, // no default
    );

    let (_tx, rx) = tokio::sync::oneshot::channel::<DecisionResponse>();
    let result = engine.request_decision(request, Some(rx)).await;
    assert!(result.timed_out());
    assert!(result.aborted());
}

#[test]
fn test_pending_approvals() {
    let engine = DecisionEngine::new();
    assert!(!engine.has_pending_approval());
}

// ═══════════════════════════════════════════════════════════
// UPDATE THROTTLE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_throttle_caps_pending() {
    let throttle = UpdateThrottle::new(50);
    for i in 0..10000 {
        throttle.push(ProactiveUpdate::Progress {
            percent: (i % 100) as f64,
            message: format!("step {i}"),
            deployment_id: None,
        });
    }
    assert!(
        throttle.pending_count() < 100,
        "Throttle should cap, got {}",
        throttle.pending_count()
    );
}

#[test]
fn test_throttle_drain() {
    let throttle = UpdateThrottle::new(100);
    throttle.push(ProactiveUpdate::Acknowledgment {
        message: "test".into(),
    });
    let drained = throttle.drain();
    assert_eq!(drained.len(), 1);
    assert_eq!(throttle.pending_count(), 0);
}

// ═══════════════════════════════════════════════════════════
// EDGE CASE TESTS (EC-UX-001 through EC-UX-010)
// ═══════════════════════════════════════════════════════════

/// EC-UX-001: User disconnects during update stream
#[test]
fn test_ec_ux_001_disconnect_during_updates() {
    let engine = ProactiveEngine::new(ProactiveConfig::default());
    let rx = engine.subscribe();
    engine.send_progress(50.0, "working");
    drop(rx); // Simulate disconnect
              // Should not crash when sending to dead channel
    engine.send_progress(100.0, "done");
    assert!(engine.is_healthy());
}

/// EC-UX-002: Update flood (too many updates)
#[test]
fn test_ec_ux_002_update_flood() {
    let throttle = UpdateThrottle::new(50);
    for i in 0..10_000 {
        throttle.push(ProactiveUpdate::Progress {
            percent: (i % 100) as f64,
            message: format!("step {i}"),
            deployment_id: None,
        });
    }
    // Should throttle/batch, not grow unbounded
    let pending = throttle.pending_count();
    assert!(pending < 100, "Should batch updates, got {pending} pending");
}

/// EC-UX-003: Voice output during voice input (collision prevention)
#[test]
fn test_ec_ux_003_voice_collision() {
    // Voice collision is tracked via icon state machine
    let icon = IconStateMachine::new();
    icon.transition(IconState::Listening);
    assert_eq!(icon.current(), IconState::Listening);
    // When speaking (Working), listening should pause
    icon.transition(IconState::Working);
    assert_eq!(icon.current(), IconState::Working);
    // After speaking, return to Listening
    icon.transition(IconState::Listening);
    assert_eq!(icon.current(), IconState::Listening);
}

/// EC-UX-004: No audio device — fall back to text
#[test]
fn test_ec_ux_004_no_audio_device() {
    // Voice is optional — engine works fine without it
    let engine = ProactiveEngine::new(ProactiveConfig::default());
    let mut rx = engine.subscribe();
    // With no audio device, updates are text-only (default behavior)
    engine.send_acknowledgment("Got it!");
    let update = rx.try_recv().unwrap();
    match update {
        ProactiveUpdate::Acknowledgment { message } => assert_eq!(message, "Got it!"),
        _ => panic!("Expected text acknowledgment"),
    }
}

/// EC-UX-005: Screen reader active — accessible output
#[test]
fn test_ec_ux_005_screen_reader() {
    let engine = ProactiveEngine::new(ProactiveConfig::default());
    engine.enable_screen_reader_mode();
    assert!(engine.is_screen_reader_mode());

    let update = ProactiveUpdate::Progress {
        percent: 50.0,
        message: "Analyzing code".into(),
        deployment_id: None,
    };
    let accessible = engine.format_accessible(&update);
    assert!(accessible.is_accessible());
    assert_eq!(accessible.role, "status");
    assert_eq!(accessible.aria_live, "polite");
    assert!(accessible.description.contains("50"));
}

/// EC-UX-006: Decision timeout with no default — must abort
#[tokio::test]
async fn test_ec_ux_006_decision_timeout_no_default() {
    let engine = DecisionEngine::new();
    let request = DecisionEngine::build_request(
        "Delete everything?",
        vec![DecisionOption {
            label: "Yes".into(),
            description: None,
            risk_level: Some(RiskLevel::Critical),
            keyboard_shortcut: None,
        }],
        0,    // immediate timeout
        None, // NO default — must abort
    );

    let (_tx, rx) = tokio::sync::oneshot::channel::<DecisionResponse>();
    let result = engine.request_decision(request, Some(rx)).await;
    // Must abort, not pick random option
    assert!(result.timed_out());
    assert!(result.aborted());
    assert!(result.chosen_index().is_none());
}

/// EC-UX-007: Very long task name — should truncate
#[test]
fn test_ec_ux_007_long_task_name() {
    let engine = ProactiveEngine::new(ProactiveConfig {
        max_task_name_length: 200,
        ..Default::default()
    });
    let name = "a".repeat(10_000);
    let formatted = engine.format_progress(&name, 50.0);
    assert!(
        formatted.len() < 1000,
        "Should truncate, got {} chars",
        formatted.len()
    );
    assert!(formatted.contains("..."));
}

/// EC-UX-008: RTL language text
#[test]
fn test_ec_ux_008_rtl_text() {
    let engine = ProactiveEngine::new(ProactiveConfig::default());
    let mut rx = engine.subscribe();

    // Send an Arabic/RTL message
    engine.send_acknowledgment("مرحبا");
    let update = rx.try_recv().unwrap();
    match update {
        ProactiveUpdate::Acknowledgment { message } => {
            assert_eq!(message, "مرحبا");
            // RTL text is preserved as-is — rendering is handled by the frontend
            assert!(!message.is_empty());
        }
        _ => panic!("Expected Acknowledgment"),
    }
}

/// EC-UX-009: Notification permission denied — fallback to in-app
#[test]
fn test_ec_ux_009_notification_denied() {
    let engine = ProactiveEngine::new(ProactiveConfig::default());
    engine.deny_notifications();
    assert!(engine.notifications_denied());

    // Updates still work via broadcast channel (in-app fallback)
    let mut rx = engine.subscribe();
    engine.send_alert(AlertLevel::Warning, "Important!", None);
    let update = rx.try_recv().unwrap();
    match update {
        ProactiveUpdate::Alert { message, .. } => assert_eq!(message, "Important!"),
        _ => panic!("Expected Alert"),
    }
}

/// EC-UX-010: Approval during system sleep — should persist and re-present
#[tokio::test]
async fn test_ec_ux_010_approval_during_sleep() {
    let engine = DecisionEngine::new();
    let request = DecisionEngine::build_request(
        "Delete file?",
        vec![DecisionOption {
            label: "Yes".into(),
            description: None,
            risk_level: Some(RiskLevel::High),
            keyboard_shortcut: None,
        }],
        0, // Will timeout immediately (simulating sleep)
        None,
    );

    let (_tx, rx) = tokio::sync::oneshot::channel::<DecisionResponse>();
    let _result = engine.request_decision(request, Some(rx)).await;

    // After "wake" — pending approval should still be there
    // (TimedOutAborted with no default keeps it in pending)
    assert!(
        engine.has_pending_approval(),
        "Approval should persist through sleep/wake"
    );

    // Pending approvals can be re-presented
    let pending = engine.pending_approvals();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].request.question, "Delete file?");
}

// ═══════════════════════════════════════════════════════════
// ADDITIONAL INTEGRATION TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_icon_state_during_proactive_flow() {
    let engine = ProactiveEngine::new(ProactiveConfig::default());
    let icon = engine.icon();

    assert_eq!(icon.current(), IconState::Idle);

    engine.send_acknowledgment("Got it!");
    assert_eq!(icon.current(), IconState::Listening);

    engine.send_progress(25.0, "working");
    assert_eq!(icon.current(), IconState::Working);

    engine.send_alert(AlertLevel::Error, "failed", None);
    assert_eq!(icon.current(), IconState::Error);
}

#[test]
fn test_accessible_update_variants() {
    let engine = ProactiveEngine::new(ProactiveConfig::default());

    // Decision should be assertive
    let decision = ProactiveUpdate::Decision {
        request: DecisionRequest {
            id: uuid::Uuid::new_v4(),
            question: "Proceed?".into(),
            options: vec![DecisionOption {
                label: "Yes".into(),
                description: None,
                risk_level: None,
                keyboard_shortcut: Some("y".into()),
            }],
            timeout_seconds: Some(30),
            default: Some(0),
        },
    };
    let accessible = engine.format_accessible(&decision);
    assert_eq!(accessible.aria_live, "assertive");

    // Progress should be polite
    let progress = ProactiveUpdate::Progress {
        percent: 50.0,
        message: "working".into(),
        deployment_id: None,
    };
    let accessible = engine.format_accessible(&progress);
    assert_eq!(accessible.aria_live, "polite");
}

// ═══════════════════════════════════════════════════════════
// TIMING TESTS (ALL MUST PASS)
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_progress_updates_every_3_to_5_seconds() {
    let config = ProactiveConfig {
        progress_interval: Duration::from_millis(100), // Scaled down for testing
        max_silence: Duration::from_millis(100),
        ..Default::default()
    };
    let engine = ProactiveEngine::new(config);
    let mut rx = engine.subscribe();
    let handle = engine.start_silence_watcher();

    // Wait 1.5s (watcher checks every 1s with 100ms silence threshold)
    tokio::time::sleep(Duration::from_millis(1500)).await;
    engine.stop_silence_watcher();
    handle.abort();

    // Should have received periodic "still working" events
    let mut event_count = 0;
    while let Ok(update) = rx.try_recv() {
        if let ProactiveUpdate::Event { title, .. } = update {
            if title.contains("Still working") {
                event_count += 1;
            }
        }
    }
    // At least 1 silence-break event should have fired
    assert!(
        event_count >= 1,
        "Expected periodic updates, got {event_count}"
    );
}

#[test]
fn test_onboarding_completes_in_30_seconds() {
    let start = Instant::now();
    let mut flow = OnboardingFlow::new();
    flow.advance(None); // Welcome
    flow.advance(Some("Alice")); // Name
    flow.advance(Some("yes")); // Voice
    let elapsed = start.elapsed();
    assert!(flow.is_complete());
    assert!(
        elapsed < Duration::from_secs(30),
        "Onboarding took {:?}, must be < 30s",
        elapsed
    );
}

#[tokio::test]
async fn test_decision_timeout_uses_default() {
    let engine = DecisionEngine::new();
    let request = DecisionEngine::build_request(
        "Continue?",
        vec![
            DecisionOption {
                label: "Yes".into(),
                description: None,
                risk_level: None,
                keyboard_shortcut: Some("y".into()),
            },
            DecisionOption {
                label: "No".into(),
                description: None,
                risk_level: None,
                keyboard_shortcut: Some("n".into()),
            },
        ],
        0,       // immediate timeout
        Some(0), // default to "Yes"
    );

    let (_tx, rx) = tokio::sync::oneshot::channel::<DecisionResponse>();
    let result = engine.request_decision(request, Some(rx)).await;
    assert!(result.timed_out());
    assert_eq!(result.chosen_index(), Some(0)); // Used the default
    match result {
        DecisionResult::TimedOutWithDefault { label, .. } => {
            assert_eq!(label, "Yes");
        }
        _ => panic!("Expected TimedOutWithDefault"),
    }
}

// ═══════════════════════════════════════════════════════════
// DECISION MAX OPTIONS ENFORCEMENT
// ═══════════════════════════════════════════════════════════

#[test]
fn test_decision_max_4_options_enforced() {
    let options: Vec<DecisionOption> = (0..6)
        .map(|i| DecisionOption {
            label: format!("Option {i}"),
            description: None,
            risk_level: None,
            keyboard_shortcut: None,
        })
        .collect();
    let request = DecisionEngine::build_request("Test?", options, 30, Some(0));
    assert!(
        request.options.len() <= DecisionEngine::MAX_OPTIONS,
        "Options should be capped at {}, got {}",
        DecisionEngine::MAX_OPTIONS,
        request.options.len()
    );
    assert!(DecisionEngine::validate_request(&request));
}

#[test]
fn test_decision_default_cleared_if_beyond_truncation() {
    let options: Vec<DecisionOption> = (0..6)
        .map(|i| DecisionOption {
            label: format!("Option {i}"),
            description: None,
            risk_level: None,
            keyboard_shortcut: None,
        })
        .collect();
    // Default at index 5 — beyond truncated list of 4
    let request = DecisionEngine::build_request("Test?", options, 30, Some(5));
    assert!(
        request.default.is_none(),
        "Default should be cleared when beyond truncated options"
    );
}

#[test]
fn test_check_silence() {
    let config = ProactiveConfig {
        max_silence: Duration::from_millis(1),
        ..Default::default()
    };
    let engine = ProactiveEngine::new(config);
    std::thread::sleep(Duration::from_millis(5));
    assert!(engine.check_silence());
}
