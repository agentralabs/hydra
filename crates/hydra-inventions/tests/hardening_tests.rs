//! Category 1: Unit Gap Fill — hydra-inventions edge cases.

use hydra_inventions::*;
use std::collections::HashMap;

// === Checkpoint corruption recovery ===

#[test]
fn test_checkpoint_store_restore_nonexistent() {
    let store = resurrection::CheckpointStore::new(100);
    assert!(store.restore("nonexistent").is_none());
}

#[test]
fn test_checkpoint_store_cleanup_empty() {
    let store = resurrection::CheckpointStore::new(100);
    store.cleanup(10); // cleanup with nothing stored
    assert_eq!(store.count(), 0);
}

#[test]
fn test_checkpoint_diff_empty() {
    let state: HashMap<String, serde_json::Value> =
        HashMap::from([("a".into(), serde_json::json!(1))]);
    let cp1 = resurrection::checkpoint::Checkpoint::create("cp1", state.clone(), None);
    let cp2 = resurrection::checkpoint::Checkpoint::create("cp2", state, None);
    let diff = cp1.diff(&cp2);
    // Same state = no changes
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
}

// === Timeline branching edge cases ===

#[test]
fn test_timeline_switch_nonexistent() {
    let timeline = resurrection::timeline::Timeline::new();
    assert!(!timeline.switch("nonexistent"));
}

#[test]
fn test_timeline_branch_from_empty() {
    let timeline = resurrection::timeline::Timeline::new();
    // Can branch even with no checkpoints (branches from empty state)
    let branch_id = timeline.branch("test-branch", "");
    assert!(!branch_id.is_empty());
}

// === Dream resource exhaustion ===

#[test]
fn test_dream_config_limits() {
    let config = dream::simulator::DreamConfig {
        enabled: true,
        max_dream_duration: std::time::Duration::from_millis(100),
        min_idle_level: dream::simulator::IdleLevel::LightIdle,
        max_resource_pct: 0.1,
        max_insights_per_session: 1,
    };
    let sim = dream::simulator::DreamSimulator::new(config);
    assert!(!sim.can_dream()); // Not idle enough
}

#[test]
fn test_insight_store_capacity() {
    let store = dream::insights::InsightStore::new(200);
    for i in 0..100 {
        let insight = dream::insights::DreamInsight::new(
            &format!("task_{}", i),
            dream::insights::InsightCategory::PatternDiscovered,
            &format!("insight {}", i),
            0.5,
        );
        store.add(insight);
    }
    assert_eq!(store.count(), 100);
}

// === Shadow divergence edge cases ===

#[test]
fn test_divergence_detector_empty() {
    let main_result: HashMap<String, serde_json::Value> = HashMap::new();
    let shadow_result: HashMap<String, serde_json::Value> = HashMap::new();
    let divergences = shadow::divergence::DivergenceDetector::detect(
        &main_result,
        &shadow_result,
        true,
        true,
        true,
        true,
    );
    assert!(divergences.is_empty());
}

#[test]
fn test_divergence_detector_extra_output() {
    let main_result: HashMap<String, serde_json::Value> =
        HashMap::from([("a".into(), serde_json::json!(1))]);
    let shadow_result: HashMap<String, serde_json::Value> = HashMap::from([
        ("a".into(), serde_json::json!(1)),
        ("b".into(), serde_json::json!(2)),
    ]);
    let divergences = shadow::divergence::DivergenceDetector::detect(
        &main_result,
        &shadow_result,
        true,
        true,
        true,
        true,
    );
    assert!(!divergences.is_empty());
}

#[test]
fn test_shadow_validator_safe_action() {
    let validator = shadow::validator::ShadowValidator::new();
    let expected = HashMap::from([(
        "shadow_output".into(),
        serde_json::json!({"action": "read", "target": "file.txt"}),
    )]);
    let result = validator.validate(
        "read file",
        serde_json::json!({"action": "read", "target": "file.txt"}),
        &expected,
    );
    assert!(result.validated);
}

// === Fork max depth ===

#[test]
fn test_fork_max_branches_enforced() {
    let mut fork = forking::fork::ForkPoint::new("test", HashMap::new()).with_max_branches(2);
    assert!(fork.add_branch("b1", vec!["action1".into()]).is_ok());
    assert!(fork.add_branch("b2", vec!["action2".into()]).is_ok());
    assert!(fork.add_branch("b3", vec!["action3".into()]).is_err()); // exceeds max
}

#[test]
fn test_fork_cancel_pending() {
    let mut fork = forking::fork::ForkPoint::new("test", HashMap::new());
    fork.add_branch("b1", vec!["a".into()]).unwrap();
    fork.add_branch("b2", vec!["b".into()]).unwrap();
    let cancelled = fork.cancel_pending();
    assert_eq!(cancelled, 2);
}

// === Future echo ===

#[test]
fn test_action_chain_total_risk() {
    let chain = future_echo::predictor::ActionChain::new(vec![
        future_echo::predictor::Action {
            name: "a".into(),
            params: serde_json::json!({}),
            risk_level: 0.3,
        },
        future_echo::predictor::Action {
            name: "b".into(),
            params: serde_json::json!({}),
            risk_level: 0.5,
        },
    ]);
    let risk = chain.total_risk();
    assert!(risk > 0.3); // compound risk > individual risks
    assert!(risk < 1.0);
}

#[test]
fn test_confidence_model_calibration() {
    let model = future_echo::confidence::ConfidenceModel::new();
    let id1 = model.record_prediction(0.8);
    model.record_outcome(id1, true);
    let id2 = model.record_prediction(0.9);
    model.record_outcome(id2, false);
    let calibrated = model.calibrate(0.8);
    assert!(calibrated.value > 0.0);
}

// === Mutation ===

#[test]
fn test_pattern_tracker_underperforming() {
    let tracker = mutation::tracker::PatternTracker::new();
    let id = tracker.register("slow", vec!["step1".into(), "step2".into()]);
    // Record failures
    for _ in 0..10 {
        tracker.record(&id, false, 1000.0);
    }
    let under = tracker.underperforming(0.5, 1);
    assert!(!under.is_empty());
}

#[test]
fn test_evolution_convergence() {
    let engine = mutation::evolution::EvolutionEngine::new(0.5);
    assert!(!engine.converged(0.01));
    assert_eq!(engine.generation_count(), 0);
}

#[test]
fn test_ab_test_insufficient_samples() {
    let tester = mutation::ab_test::ABTester::new(0.95);
    let test_id = tester.create_test(
        "test1",
        mutation::ab_test::Variant::new("a", vec!["a".into()]),
        mutation::ab_test::Variant::new("b", vec!["b".into()]),
        100, // need 100 samples
    );
    assert!(tester.winner(&test_id).is_none()); // not enough samples
}

// === Minimizer ===

#[test]
fn test_dedup_empty_input() {
    let dedup = minimizer::dedup::SemanticDedup::new(0.9, 10);
    let result = dedup.deduplicate("");
    assert_eq!(result.original_tokens, 0);
    assert_eq!(result.duplicates_found, 0);
}

#[test]
fn test_compressor_aggressive() {
    let compressor = minimizer::compressor::ContextCompressor::new(
        minimizer::compressor::CompressionLevel::Aggressive,
    );
    let long_text = "This is a test sentence. ".repeat(100);
    let result = compressor.compress(&long_text);
    assert!(result.compression_ratio() < 1.0); // should compress
}

#[test]
fn test_reference_substitution_empty() {
    let sub = minimizer::reference::ReferenceSubstitution::new(20, 2);
    let (_, map) = sub.substitute("");
    assert!(map.is_empty());
}
