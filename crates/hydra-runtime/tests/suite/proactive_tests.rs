use hydra_runtime::proactive::{
    AlertSeverity, DecisionOption, EventType, ProactiveEngine, ProactiveUpdate,
};

#[test]
fn test_engine_default_config() {
    let engine = ProactiveEngine::new();
    assert_eq!(engine.max_silence_ms(), 5000);
    assert_eq!(engine.progress_interval_ms(), 3000);
    assert_eq!(engine.update_count(), 0);
}

#[test]
fn test_engine_custom_config() {
    let engine = ProactiveEngine::with_config(10000, 1000);
    assert_eq!(engine.max_silence_ms(), 10000);
    assert_eq!(engine.progress_interval_ms(), 1000);
}

#[test]
fn test_acknowledge() {
    let mut engine = ProactiveEngine::new();
    engine.acknowledge("Got it, working on it");
    assert_eq!(engine.update_count(), 1);
    match engine.last_update() {
        Some(ProactiveUpdate::Acknowledgment { message, .. }) => {
            assert_eq!(message, "Got it, working on it");
        }
        other => panic!("Expected Acknowledgment, got {:?}", other),
    }
}

#[test]
fn test_progress() {
    let mut engine = ProactiveEngine::new();
    engine.progress(50.0, "Compiling", 3);
    match engine.last_update() {
        Some(ProactiveUpdate::Progress {
            percent,
            current_step,
            steps_remaining,
        }) => {
            assert!((percent - 50.0).abs() < f32::EPSILON);
            assert_eq!(current_step, "Compiling");
            assert_eq!(*steps_remaining, 3);
        }
        other => panic!("Expected Progress, got {:?}", other),
    }
}

#[test]
fn test_complete() {
    let mut engine = ProactiveEngine::new();
    engine.complete(
        "Build finished",
        vec!["compiled 10 files".into()],
        vec!["run tests".into()],
    );
    match engine.last_update() {
        Some(ProactiveUpdate::Completion {
            summary,
            changes,
            next_steps,
        }) => {
            assert_eq!(summary, "Build finished");
            assert_eq!(changes.len(), 1);
            assert_eq!(next_steps.len(), 1);
        }
        other => panic!("Expected Completion, got {:?}", other),
    }
}

#[test]
fn test_alert() {
    let mut engine = ProactiveEngine::new();
    engine.alert(AlertSeverity::Warning, "Disk space low", true);
    match engine.last_update() {
        Some(ProactiveUpdate::Alert {
            severity,
            message,
            recoverable,
            ..
        }) => {
            assert_eq!(*severity, AlertSeverity::Warning);
            assert_eq!(message, "Disk space low");
            assert!(*recoverable);
        }
        other => panic!("Expected Alert, got {:?}", other),
    }
}

#[test]
fn test_drain_clears_updates() {
    let mut engine = ProactiveEngine::new();
    engine.acknowledge("hello");
    engine.progress(25.0, "step1", 3);
    engine.progress(50.0, "step2", 2);

    let drained = engine.drain();
    assert_eq!(drained.len(), 3);
    assert_eq!(engine.update_count(), 0);
    assert!(engine.last_update().is_none());
}

#[test]
fn test_check_silence_no_updates() {
    let engine = ProactiveEngine::new();
    assert!(!engine.check_silence(10000));
}

#[test]
fn test_check_silence_within_limit() {
    let mut engine = ProactiveEngine::new();
    engine.acknowledge("hi");
    // The last_update_ms is set to current time, so checking with
    // a time very close to now should not trigger silence
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    assert!(!engine.check_silence(now + 1000));
}

#[test]
fn test_check_silence_exceeded() {
    let mut engine = ProactiveEngine::with_config(100, 50);
    engine.acknowledge("hi");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    // Check well beyond the 100ms silence limit
    assert!(engine.check_silence(now + 200));
}

#[test]
fn test_send_custom_update() {
    let mut engine = ProactiveEngine::new();
    engine.send(ProactiveUpdate::Event {
        event_type: EventType::Discovery,
        description: "Found 3 related files".into(),
        requires_attention: false,
    });
    match engine.last_update() {
        Some(ProactiveUpdate::Event {
            event_type,
            description,
            requires_attention,
        }) => {
            assert_eq!(*event_type, EventType::Discovery);
            assert_eq!(description, "Found 3 related files");
            assert!(!requires_attention);
        }
        other => panic!("Expected Event, got {:?}", other),
    }
}

#[test]
fn test_decision_option_builder() {
    let opt = DecisionOption::new("Yes", "Proceed with changes").with_shortcut('y');
    assert_eq!(opt.label, "Yes");
    assert_eq!(opt.description, "Proceed with changes");
    assert_eq!(opt.keyboard_shortcut, Some('y'));
}

#[test]
fn test_send_decision() {
    let mut engine = ProactiveEngine::new();
    engine.send(ProactiveUpdate::Decision {
        question: "Overwrite existing file?".into(),
        options: vec![
            DecisionOption::new("Yes", "Overwrite").with_shortcut('y'),
            DecisionOption::new("No", "Skip").with_shortcut('n'),
        ],
        timeout_secs: 30,
        default: Some(1),
    });
    match engine.last_update() {
        Some(ProactiveUpdate::Decision {
            question,
            options,
            timeout_secs,
            default,
        }) => {
            assert_eq!(question, "Overwrite existing file?");
            assert_eq!(options.len(), 2);
            assert_eq!(*timeout_secs, 30);
            assert_eq!(*default, Some(1));
        }
        other => panic!("Expected Decision, got {:?}", other),
    }
}

#[test]
fn test_alert_severity_ordering() {
    assert!(AlertSeverity::Info < AlertSeverity::Warning);
    assert!(AlertSeverity::Warning < AlertSeverity::Error);
    assert!(AlertSeverity::Error < AlertSeverity::Critical);
}

#[test]
fn test_multiple_updates_sequence() {
    let mut engine = ProactiveEngine::new();
    engine.acknowledge("Starting");
    engine.progress(0.0, "Init", 5);
    engine.progress(20.0, "Phase 1", 4);
    engine.progress(40.0, "Phase 2", 3);
    engine.alert(AlertSeverity::Info, "Cache miss", true);
    engine.progress(60.0, "Phase 3", 2);
    engine.progress(80.0, "Phase 4", 1);
    engine.complete("All done", vec![], vec![]);
    assert_eq!(engine.update_count(), 8);
}
