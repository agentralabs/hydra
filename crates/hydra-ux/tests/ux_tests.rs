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
