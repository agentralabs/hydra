//! Integration tests for hydra-inventions — resurrection, dream, shadow, token minimizer.

use std::collections::HashMap;

// === Resurrection Tests ===

#[test]
fn test_checkpoint_save_restore() {
    use hydra_inventions::resurrection::{Checkpoint, CheckpointStore};

    let store = CheckpointStore::new(10);
    let mut state = HashMap::new();
    state.insert("counter".into(), serde_json::json!(42));
    state.insert("name".into(), serde_json::json!("hydra"));

    let checkpoint = Checkpoint::create("test-save", state.clone(), None);
    assert!(!checkpoint.hash.is_empty());

    let id = checkpoint.id.clone();
    store.save(checkpoint);
    let restored = store.restore(&id).unwrap();
    assert_eq!(restored.state.get("counter"), Some(&serde_json::json!(42)));
}

#[test]
fn test_timeline_branching() {
    use hydra_inventions::resurrection::Timeline;

    let timeline = Timeline::new();
    let branch_id = timeline.branch("experiment-1", "cp-0");

    assert!(timeline.switch(&branch_id));
    assert_eq!(timeline.current().name, "experiment-1");

    // Branch from branch
    let sub_branch = timeline.branch("experiment-1a", "cp-1");
    assert!(timeline.switch(&sub_branch));
    assert_eq!(timeline.current().name, "experiment-1a");
}

#[test]
fn test_replay_with_modifications() {
    use hydra_inventions::resurrection::{
        replay::ModAction, Checkpoint, ReplayModification, Replayer,
    };

    let mut state = HashMap::new();
    state.insert("x".into(), serde_json::json!(10));
    state.insert("y".into(), serde_json::json!(20));

    let cp = Checkpoint::create("orig", state, None);

    let mods = vec![
        ReplayModification {
            key: "x".into(),
            action: ModAction::Set(serde_json::json!(99)),
        },
        ReplayModification {
            key: "y".into(),
            action: ModAction::Remove,
        },
    ];

    let result = Replayer::replay(&cp, &mods);
    assert_eq!(result.final_state.get("x"), Some(&serde_json::json!(99)));
    assert!(!result.final_state.contains_key("y"));
}

// === Dream State Tests ===

#[test]
fn test_dream_simulator_idle_levels() {
    use hydra_inventions::dream::{DreamConfig, DreamSimulator, IdleLevel};

    let sim = DreamSimulator::new(DreamConfig::default());

    // Active = can't dream, no eligible tasks
    assert!(!sim.can_dream());
    let eligible = sim.eligible_tasks();
    assert!(eligible.is_empty());

    // LightIdle = some tasks
    sim.set_idle_level(IdleLevel::LightIdle);
    assert!(sim.can_dream());
    let eligible = sim.eligible_tasks();
    assert!(!eligible.is_empty());

    // DeepIdle = more tasks
    sim.set_idle_level(IdleLevel::DeepIdle);
    let eligible = sim.eligible_tasks();
    assert!(eligible.len() >= 3);
}

#[test]
fn test_dream_insight_store() {
    use hydra_inventions::dream::{DreamInsight, InsightCategory, InsightStore};

    let store = InsightStore::new(100);

    store.add(DreamInsight::new(
        "pattern_mining",
        InsightCategory::PatternDiscovered,
        "Users prefer short responses",
        0.9,
    ));

    store.add(DreamInsight::new(
        "optimization_scan",
        InsightCategory::OptimizationFound,
        "Cache miss rate is high",
        0.7,
    ));

    let unsurfaced = store.surface(0.5);
    assert_eq!(unsurfaced.len(), 2);

    // After surfacing, they should be marked
    let unsurfaced_again = store.surface(0.5);
    assert!(unsurfaced_again.is_empty());
}

#[test]
fn test_alternative_explorer() {
    use hydra_inventions::dream::{AlternativeExplorer, Scenario};

    let explorer = AlternativeExplorer::new();
    let scenario = Scenario {
        id: "s1".into(),
        description: "What if we used a different model?".into(),
        original_action: "haiku_classify".into(),
        alternative_action: "sonnet_classify".into(),
        context: serde_json::json!({"model": "haiku"}),
    };

    let result = explorer.explore(scenario);
    assert_eq!(result.scenario_id, "s1");
    assert!(result.confidence > 0.0);
}

#[test]
fn test_dream_session_produces_insights() {
    use hydra_inventions::dream::{DreamConfig, DreamSimulator, IdleLevel};

    let sim = DreamSimulator::new(DreamConfig::default());
    sim.set_idle_level(IdleLevel::DeepIdle);

    let insights = sim.dream_session();
    assert!(!insights.is_empty());
    assert_eq!(sim.session_count(), 1);
    assert_eq!(sim.insights().count(), insights.len());
}

// === Shadow Self Tests ===

#[test]
fn test_shadow_execution_and_divergence() {
    use hydra_inventions::shadow::executor::ShadowStatus;
    use hydra_inventions::shadow::{DivergenceDetector, ShadowExecutor};

    let executor = ShadowExecutor::default();
    let run = executor.execute("test action", serde_json::json!({"key": "value"}));
    assert_eq!(run.status, ShadowStatus::Running);

    let result = executor.result(&run.id).unwrap();

    // Compare with different expected output to get divergences
    let mut expected = HashMap::new();
    expected.insert("different_key".into(), serde_json::json!("different_value"));

    let divergences = DivergenceDetector::detect(
        &expected,
        &result.outputs,
        result.success,
        result.success,
        result.safe,
        result.safe,
    );
    assert!(!divergences.is_empty());
}

#[test]
fn test_shadow_validator_safe() {
    use hydra_inventions::shadow::validator::Recommendation;
    use hydra_inventions::shadow::ShadowValidator;

    let validator = ShadowValidator::new();
    let expected = HashMap::from([("shadow_output".into(), serde_json::json!({"test": true}))]);

    let outcome = validator.validate("safe op", serde_json::json!({"test": true}), &expected);
    assert!(outcome.validated);
    assert!(outcome.safe);
    assert_eq!(outcome.recommendation, Recommendation::Proceed);
}

// === Token Minimizer Tests ===

#[test]
fn test_dedup_removes_duplicates() {
    use hydra_inventions::minimizer::SemanticDedup;

    let dedup = SemanticDedup::new(0.9, 5);
    let content = "This is a sufficiently long line for dedup testing purposes\n\
                   Some unique middle content here\n\
                   This is a sufficiently long line for dedup testing purposes\n\
                   Another unique line\n\
                   This is a sufficiently long line for dedup testing purposes";

    let result = dedup.deduplicate(content);
    assert_eq!(result.duplicates_found, 2);
    assert!(result.compression_ratio() > 0.1);
}

#[test]
fn test_compressor_achieves_compression() {
    use hydra_inventions::minimizer::{CompressionLevel, ContextCompressor};

    let compressor = ContextCompressor::new(CompressionLevel::Aggressive);
    let verbose =
        "function   hello()   {\n\n\n\n    return    true;\n\n\n}\n\n\n\nconst   x   =   1;\n\n\n";
    let result = compressor.compress(verbose);
    assert!(result.compression_ratio() > 0.2);
    assert!(result.compressed_tokens < result.original_tokens);
}

#[test]
fn test_reference_substitution() {
    use hydra_inventions::minimizer::ReferenceSubstitution;

    let sub = ReferenceSubstitution::new(15, 2);
    let content = "This repeated content is long enough to substitute\n\
                   unique line here\n\
                   This repeated content is long enough to substitute\n\
                   another unique line\n\
                   This repeated content is long enough to substitute";

    let (result, map) = sub.substitute(content);
    assert!(!map.is_empty());
    assert!(result.contains("$ref_"));

    // Verify expansion restores content
    let expanded = map.expand(&result);
    assert!(!expanded.contains("$ref_"));
}

#[test]
fn test_full_minimizer_pipeline() {
    use hydra_inventions::minimizer::{
        CompressionLevel, ContextCompressor, ReferenceSubstitution, SemanticDedup,
    };

    let input = "function   processData()   {\n\
                 \n\
                 \n\
                     return   processData_result;\n\
                 }\n\
                 \n\
                 \n\
                 function   processData()   {\n\
                 \n\
                 \n\
                     return   processData_result;\n\
                 }";

    // Step 1: Dedup
    let dedup = SemanticDedup::new(0.9, 5);
    let deduped = dedup.deduplicate(input);

    // Step 2: Compress
    let compressor = ContextCompressor::new(CompressionLevel::Medium);
    let compressed = compressor.compress(&deduped.content);

    // Step 3: Reference substitution
    let sub = ReferenceSubstitution::new(15, 2);
    let (final_result, _map) = sub.substitute(&compressed.content);

    // Pipeline should reduce tokens
    let original_tokens = (input.len() + 3) / 4;
    let final_tokens = (final_result.len() + 3) / 4;
    assert!(final_tokens < original_tokens);
}
