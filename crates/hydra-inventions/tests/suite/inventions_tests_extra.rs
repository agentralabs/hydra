//! Integration tests for hydra-inventions — future echo, mutation, forking.

use std::collections::HashMap;

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
