//! Category 5: Stress — edge case inputs.

#[test]
fn test_empty_input_handling() {
    use hydra_core::*;
    let intent = Intent::new("", IntentSource::Cli);
    assert!(intent.text.is_empty());
}

#[test]
fn test_max_length_input() {
    use hydra_core::*;
    let huge = "x".repeat(1_000_000); // 1MB
    let intent = Intent::new(&huge, IntentSource::Api);
    assert_eq!(intent.text.len(), 1_000_000);
}

#[test]
fn test_unicode_edge_cases() {
    use hydra_core::*;

    let inputs = vec![
        "Hello 🌍",
        "Ñoño señor",
        "日本語テスト",
        "مرحبا",
        "🦀🦀🦀",
        "\u{200B}\u{200B}", // zero-width spaces
        "a\nb\nc", // newlines
        "a\t\tb", // tabs
    ];

    for input in inputs {
        let intent = Intent::new(input, IntentSource::Cli);
        // Should not panic
        let _ = serde_json::to_string(&intent).unwrap();
    }
}

#[test]
fn test_deeply_nested_json() {
    // Build deeply nested JSON
    let mut val = serde_json::json!({"value": "leaf"});
    for _ in 0..100 {
        val = serde_json::json!({"nested": val});
    }
    // Should serialize without stack overflow
    let s = serde_json::to_string(&val).unwrap();
    assert!(!s.is_empty());
}

#[test]
fn test_large_checkpoint_state() {
    use hydra_inventions::resurrection::checkpoint::Checkpoint;

    let mut data = serde_json::Map::new();
    for i in 0..1000 {
        data.insert(format!("key_{}", i), serde_json::json!(format!("value_{}", i)));
    }
    let cp = Checkpoint::create("large", serde_json::Value::Object(data));
    assert!(!cp.id.is_empty());
    assert!(cp.size_bytes > 0);
}

#[test]
fn test_many_fork_branches() {
    use hydra_inventions::forking::fork::ForkPoint;

    let mut fork = ForkPoint::new("many_branches").with_max_branches(100);
    for i in 0..100 {
        fork.add_branch(&format!("branch_{}", i), vec![format!("action_{}", i)]).unwrap();
    }
    assert_eq!(fork.active_branches(), 100);
}

#[test]
fn test_rapid_circuit_breaker_transitions() {
    use hydra_model::circuit_breaker::*;

    let cb = CircuitBreaker::new();
    for _ in 0..100 {
        cb.track_failure();
        cb.track_failure();
        cb.track_failure();
        cb.track_failure();
        cb.track_failure(); // → open
        cb.reset(); // → closed
    }
    assert_eq!(cb.state(), CircuitState::Closed);
}
