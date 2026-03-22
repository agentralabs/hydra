//! Integration tests for hydra-prediction.

use hydra_belief::BeliefStore;
use hydra_prediction::{
    compute_divergence, ActualOutcome, DivergenceDetector, IntentPrediction, PredictionBasis,
    PredictionStage, PredictionStager, ShadowOutcome,
};

#[test]
fn end_to_end_prediction_cycle() {
    let mut stager = PredictionStager::new();

    // Record a pattern of intents
    for _ in 0..5 {
        stager.record_intent("check build status", Some("ci-pipeline".into()));
    }
    stager.record_intent("deploy to staging", Some("ci-pipeline".into()));

    stager.run_cycle();
    let best = stager.stage().best();
    assert!(best.is_some(), "should have at least one prediction");
}

#[test]
fn divergence_triggers_belief_update() {
    let detector = DivergenceDetector::new();
    let shadow = ShadowOutcome {
        description: "predicted fast build".into(),
        confidence: 0.8,
        state_changes: vec![
            ("build_time".into(), "30s".into()),
            ("test_pass".into(), "true".into()),
        ],
    };
    let actual = ActualOutcome {
        description: "slow build".into(),
        state_changes: vec![
            ("build_time".into(), "300s".into()),
            ("test_pass".into(), "false".into()),
        ],
    };

    let mut store = BeliefStore::new();
    let result = detector.evaluate(&shadow, &actual, &mut store, "build-speed");
    assert!(result.is_err(), "full divergence should exceed threshold");
    assert!(
        store.len() > 0,
        "belief store should have corrective belief"
    );
}

#[test]
fn shadow_outcome_comparison() {
    let shadow = ShadowOutcome {
        description: "test".into(),
        confidence: 0.8,
        state_changes: vec![
            ("a".into(), "1".into()),
            ("b".into(), "2".into()),
            ("c".into(), "3".into()),
        ],
    };
    let actual = ActualOutcome {
        description: "test".into(),
        state_changes: vec![
            ("a".into(), "1".into()),
            ("b".into(), "different".into()),
            ("c".into(), "3".into()),
        ],
    };
    let div = compute_divergence(&shadow, &actual);
    // 1 out of 3 keys diverged
    assert!((div.score - 1.0 / 3.0).abs() < 0.01);
    assert_eq!(div.diverged_keys.len(), 1);
}

#[test]
fn prediction_stage_ordering() {
    let mut stage = PredictionStage::new();
    stage.update(IntentPrediction::new(
        "low",
        0.3,
        PredictionBasis::TemporalPattern,
    ));
    stage.update(IntentPrediction::new(
        "mid",
        0.6,
        PredictionBasis::SessionPattern,
    ));
    stage.update(IntentPrediction::new(
        "high",
        0.9,
        PredictionBasis::ActionConsequence,
    ));
    stage.update(IntentPrediction::new(
        "highest",
        0.95,
        PredictionBasis::ActiveTaskState,
    ));

    let top = stage.top();
    assert_eq!(top.len(), 3); // slot count is 3
    assert_eq!(top[0].description, "highest");
}
