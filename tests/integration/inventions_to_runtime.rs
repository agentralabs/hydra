//! Category 2: Integration — inventions ↔ runtime data flow.

use hydra_inventions::*;

#[test]
fn test_resurrection_checkpoint_lifecycle() {
    let store = resurrection::store::CheckpointStore::new();
    let cp = resurrection::checkpoint::Checkpoint::create(
        "mid_run_save",
        serde_json::json!({
            "phase": "think",
            "intent": "explain quantum computing",
            "tokens_used": 500,
        }),
    );
    store.save(cp.clone());
    let restored = store.restore(&cp.id).unwrap();
    assert_eq!(restored.label, "mid_run_save");
}

#[test]
fn test_dream_background_execution() {
    let config = dream::simulator::DreamConfig {
        enabled: true,
        max_dream_duration: std::time::Duration::from_secs(1),
        min_idle_level: dream::simulator::IdleLevel::DeepIdle,
        max_resource_pct: 0.5,
        max_insights_per_session: 10,
    };
    let sim = dream::simulator::DreamSimulator::new(config);
    sim.set_idle_level(dream::simulator::IdleLevel::DeepIdle);
    assert!(sim.can_dream());

    let insights = sim.dream_session();
    // May or may not produce insights, but shouldn't panic
    let _ = insights;
}

#[test]
fn test_shadow_parallel_validation() {
    let validator = shadow::validator::ShadowValidator::new();
    let result = validator.validate(serde_json::json!({
        "action": "write_file",
        "target": "src/lib.rs",
        "content": "fn main() {}"
    }));
    assert!(result.validated);
}

#[test]
fn test_fork_merge_to_main() {
    let mut fork = forking::fork::ForkPoint::new("api_approach");
    fork.add_branch("rest", vec!["scaffold REST".into()]).unwrap();
    fork.add_branch("graphql", vec!["scaffold GraphQL".into()]).unwrap();

    let executor = forking::parallel::ParallelExecutor::new();
    let results = executor.execute(&fork);
    assert_eq!(results.len(), 2);

    let comparator = forking::compare::OutcomeComparator::new();
    let comparison = comparator.compare(&results);
    assert!(!comparison.rankings.is_empty());

    let merger = forking::merge::ResultMerger::new(forking::merge::MergeStrategy::BestOnly);
    let merged = merger.merge(&results, &comparison);
    assert!(merged.primary.is_some());
}
