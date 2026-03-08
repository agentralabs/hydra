//! Integration tests for hydra-inventions.

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

// === Future Echo Tests ===

#[test]
fn test_outcome_prediction() {
    use hydra_inventions::future_echo::predictor::*;

    let predictor = OutcomePredictor::new();
    let chain = ActionChain::new(vec![
        Action {
            name: "read".into(),
            params: serde_json::json!({}),
            risk_level: 0.05,
        },
        Action {
            name: "transform".into(),
            params: serde_json::json!({}),
            risk_level: 0.15,
        },
        Action {
            name: "write".into(),
            params: serde_json::json!({}),
            risk_level: 0.1,
        },
    ]);

    let outcomes = predictor.predict(&chain);
    assert!(outcomes.len() >= 2);
    assert!(outcomes[0].confidence.value > 0.0);
    assert!(predictor.prediction_count() > 0);
}

#[test]
fn test_confidence_scoring() {
    use hydra_inventions::future_echo::confidence::*;

    let model = ConfidenceModel::new();
    let id1 = model.record_prediction(0.9);
    model.record_outcome(id1, true);
    let id2 = model.record_prediction(0.8);
    model.record_outcome(id2, true);
    let id3 = model.record_prediction(0.3);
    model.record_outcome(id3, false);

    let accuracy = model.accuracy().unwrap();
    assert_eq!(accuracy, 1.0);

    let score = ConfidenceScore::new(0.85);
    assert!(score.is_high());
}

#[test]
fn test_what_if_query() {
    use hydra_inventions::future_echo::predictor::Action;
    use hydra_inventions::future_echo::query::*;

    let engine = FutureQueryEngine::new();
    let query = FutureQuery::new(
        "What if I refactor the database layer?",
        vec![
            Action {
                name: "backup".into(),
                params: serde_json::json!({}),
                risk_level: 0.05,
            },
            Action {
                name: "refactor".into(),
                params: serde_json::json!({}),
                risk_level: 0.4,
            },
            Action {
                name: "test".into(),
                params: serde_json::json!({}),
                risk_level: 0.1,
            },
        ],
    );

    let result = engine.query(&query);
    assert!(!result.outcomes.is_empty());
    assert!(result.best_outcome.is_some());
    assert!(!result.recommended_action.is_empty());
}

#[test]
fn test_risk_assessment() {
    use hydra_inventions::future_echo::predictor::*;

    let predictor = OutcomePredictor::new();
    let risky = ActionChain::new(vec![Action {
        name: "delete_production".into(),
        params: serde_json::json!({}),
        risk_level: 0.9,
    }]);

    let outcomes = predictor.predict(&risky);
    assert_eq!(
        outcomes[0].risk_assessment.recommendation,
        RiskRecommendation::Block
    );
}

// === Mutation Tests ===

#[test]
fn test_pattern_tracking_and_success_rate() {
    use hydra_inventions::mutation::tracker::PatternTracker;

    let tracker = PatternTracker::new();
    let id = tracker.register(
        "deploy_flow",
        vec!["build".into(), "test".into(), "deploy".into()],
    );

    for _ in 0..7 {
        tracker.record(&id, true, 100.0);
    }
    for _ in 0..3 {
        tracker.record(&id, false, 150.0);
    }

    let pattern = tracker.get(&id).unwrap();
    assert_eq!(pattern.total_executions, 10);
    assert!((pattern.success_rate() - 0.7).abs() < f64::EPSILON);
}

#[test]
fn test_pattern_mutation_variants() {
    use hydra_inventions::mutation::mutator::{MutationType, PatternMutator};
    use hydra_inventions::mutation::tracker::PatternRecord;

    let mutator = PatternMutator::new();
    let pattern = PatternRecord::new(
        "pipeline",
        vec!["fetch".into(), "parse".into(), "store".into()],
    );

    let mutations = mutator.mutate(&pattern);
    assert!(!mutations.is_empty());

    let types: Vec<_> = mutations.iter().map(|m| &m.mutation_type).collect();
    assert!(types.contains(&&MutationType::Reorder));
    assert!(types.contains(&&MutationType::RemoveStep));
    assert!(types.contains(&&MutationType::AddStep));
}

#[test]
fn test_ab_test_winner() {
    use hydra_inventions::mutation::ab_test::*;

    let tester = ABTester::new(0.8);
    let a = Variant::new("original", vec!["step1".into(), "step2".into()]);
    let b = Variant::new(
        "mutated",
        vec!["step1".into(), "validate".into(), "step2".into()],
    );

    let test_id = tester.create_test("pipeline_ab", a, b, 5);

    // Original: 90% success
    for _ in 0..9 {
        tester.record_result(&test_id, "original", true, 80.0);
    }
    tester.record_result(&test_id, "original", false, 80.0);

    // Mutated: 40% success
    for _ in 0..4 {
        tester.record_result(&test_id, "mutated", true, 120.0);
    }
    for _ in 0..6 {
        tester.record_result(&test_id, "mutated", false, 120.0);
    }

    let winner = tester.winner(&test_id);
    assert_eq!(winner, Some("original".into()));
}

#[test]
fn test_evolution_selection() {
    use hydra_inventions::mutation::evolution::EvolutionEngine;
    use hydra_inventions::mutation::tracker::PatternRecord;

    let engine = EvolutionEngine::new(0.5);
    let mut patterns = Vec::new();

    for (name, success, fail) in &[("good", 9, 1), ("ok", 6, 4), ("bad", 2, 8)] {
        let mut p = PatternRecord::new(name, vec![name.to_string()]);
        for _ in 0..*success {
            p.record_execution(true, 100.0);
        }
        for _ in 0..*fail {
            p.record_execution(false, 100.0);
        }
        patterns.push(p);
    }

    let gen = engine.evolve(patterns);
    assert!(gen.best_fitness > 0.0);
    assert!(gen.patterns[0].success_rate() >= gen.patterns.last().unwrap().success_rate());
}

// === Forking Tests ===

#[test]
fn test_fork_creation_and_limits() {
    use hydra_inventions::forking::fork::ForkPoint;

    let mut fork =
        ForkPoint::new("decision", std::collections::HashMap::new()).with_max_branches(3);

    fork.add_branch("a", vec!["x".into()]).unwrap();
    fork.add_branch("b", vec!["y".into()]).unwrap();
    fork.add_branch("c", vec!["z".into()]).unwrap();
    assert!(fork.add_branch("d", vec![]).is_err()); // limit reached

    assert_eq!(fork.branches.len(), 3);
    assert_eq!(fork.active_branches(), 3);
}

#[test]
fn test_parallel_execution() {
    use hydra_inventions::forking::fork::ForkPoint;
    use hydra_inventions::forking::parallel::ParallelExecutor;

    let executor = ParallelExecutor::new(4);
    let mut fork = ForkPoint::new("test fork", std::collections::HashMap::new());
    fork.add_branch("approach_a", vec!["read".into(), "process".into()])
        .unwrap();
    fork.add_branch("approach_b", vec!["fetch".into()]).unwrap();

    let results = executor.execute(&mut fork);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.success));
}

#[test]
fn test_outcome_comparison_and_merge() {
    use hydra_inventions::forking::compare::OutcomeComparator;
    use hydra_inventions::forking::merge::{MergeStrategy, ResultMerger};
    use hydra_inventions::forking::parallel::BranchResult;

    let results = vec![
        BranchResult {
            branch_id: "b1".into(),
            branch_name: "fast".into(),
            success: true,
            output: serde_json::json!({"data": "fast"}),
            duration_ms: 50,
            resources_used: 0.1,
        },
        BranchResult {
            branch_id: "b2".into(),
            branch_name: "thorough".into(),
            success: true,
            output: serde_json::json!({"data": "thorough"}),
            duration_ms: 300,
            resources_used: 0.5,
        },
    ];

    let comparison = OutcomeComparator::new().compare(&results);
    assert_eq!(comparison.best_name, "fast");
    assert!(comparison.unanimous_success);

    let merger = ResultMerger::new(MergeStrategy::BestWithFallbacks);
    let merged = merger.merge(&results, &comparison);
    assert_eq!(merged.primary.branch_name, "fast");
    assert_eq!(merged.fallbacks.len(), 1);
    assert_eq!(merged.branches_used, 2);
}

#[test]
fn test_fork_cleanup() {
    use hydra_inventions::forking::fork::{BranchStatus, ForkPoint};

    let mut fork = ForkPoint::new("cleanup test", std::collections::HashMap::new());
    fork.add_branch("a", vec![]).unwrap();
    fork.add_branch("b", vec![]).unwrap();
    fork.add_branch("c", vec![]).unwrap();
    fork.branches[0].status = BranchStatus::Running;

    let cancelled = fork.cancel_pending();
    assert_eq!(cancelled, 2);
    assert_eq!(fork.active_branches(), 1);
}
