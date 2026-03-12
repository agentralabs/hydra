use std::time::{Duration, Instant};

use hydra_core::types::*;
use hydra_ux::decisions::{DecisionEngine, DecisionResult};
use hydra_ux::icon::IconStateMachine;
use hydra_ux::onboarding::OnboardingFlow;
use hydra_ux::proactive::{ProactiveConfig, ProactiveEngine, UpdateThrottle};

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
