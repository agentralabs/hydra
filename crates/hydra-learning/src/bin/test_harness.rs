//! Test harness for hydra-learning — validates observation and tracking.

use hydra_learning::observation::ObservationOutcome;
use hydra_learning::{LearningEngine, ModeTracker};
use hydra_reasoning::conclusion::{ReasoningConclusion, ReasoningMode};
use hydra_reasoning::ReasoningResult;

fn main() {
    let mut passed = 0;
    let mut failed = 0;

    macro_rules! check {
        ($name:expr, $cond:expr) => {
            if $cond {
                passed += 1;
                println!("  PASS: {}", $name);
            } else {
                failed += 1;
                println!("  FAIL: {}", $name);
            }
        };
    }

    println!("=== hydra-learning test harness ===\n");

    // 1. Observation recording
    println!("[1] Observation recording");
    let mut engine = LearningEngine::new();
    let result = make_result(ReasoningMode::Deductive);
    let obs = engine.observe(&result, "engineering", "action");
    check!("observation has id", !obs.id.is_empty());
    check!("observation has domain", obs.domain == "engineering");
    check!("observation count is 1", engine.observation_count() == 1);

    // 2. Mode accuracy tracking
    println!("\n[2] Mode accuracy tracking");
    let mut tracker = ModeTracker::new();
    for _ in 0..6 {
        let obs = hydra_learning::ReasoningObservation::from_result(
            &result,
            "engineering",
            "action",
            ObservationOutcome::Correct,
        );
        tracker.record(&obs);
    }
    let acc = tracker.accuracy_for("engineering", "deductive");
    check!("accuracy is Some", acc.is_some());
    check!(
        "accuracy is 1.0 for all-correct",
        (acc.unwrap_or(0.0) - 1.0).abs() < f64::EPSILON
    );

    // 3. Insufficient observations returns empty
    println!("\n[3] Insufficient observations");
    let mut engine2 = LearningEngine::new();
    engine2.observe_with_outcome(
        &result,
        "engineering",
        "action",
        ObservationOutcome::Correct,
    );
    let records = engine2.check_adjustments("engineering");
    check!("empty records for insufficient data", records.is_empty());

    // 4. Accuracy computation
    println!("\n[4] Accuracy computation");
    let mut tracker2 = ModeTracker::new();
    for _ in 0..3 {
        tracker2.record(&hydra_learning::ReasoningObservation::from_result(
            &result,
            "finance",
            "query",
            ObservationOutcome::Correct,
        ));
    }
    tracker2.record(&hydra_learning::ReasoningObservation::from_result(
        &result,
        "finance",
        "query",
        ObservationOutcome::Incorrect,
    ));
    let acc2 = tracker2.accuracy_for("finance", "deductive");
    check!(
        "accuracy is 0.75",
        (acc2.unwrap_or(0.0) - 0.75).abs() < f64::EPSILON
    );

    // 5. Summary format
    println!("\n[5] Summary format");
    let summary = engine.summary();
    check!("contains 'learning:'", summary.contains("learning:"));
    check!(
        "contains 'observations='",
        summary.contains("observations=")
    );
    check!("contains 'domains='", summary.contains("domains="));

    println!(
        "\n=== Learning Results: {} passed, {} failed ===",
        passed, failed
    );
    if failed > 0 {
        std::process::exit(1);
    }
}

fn make_result(mode: ReasoningMode) -> ReasoningResult {
    let conclusion = ReasoningConclusion::new(mode.clone(), "test conclusion", 0.8, vec![], false);
    ReasoningResult {
        conclusions: vec![conclusion.clone()],
        synthesis_confidence: 0.8,
        used_llm: false,
        active_modes: 1,
        primary: Some(conclusion),
        mode_summary: vec![(mode.label().to_string(), true)],
    }
}
