//! Test harness for hydra-context.
//!
//! Runs scenarios covering all 5 context windows and prints results.

use hydra_comprehension::{
    ComprehendedInput, ConstraintStatus, Domain, Horizon, InputSource, ResonanceResult,
    TemporalContext,
};
use hydra_context::{
    build_active, build_historical, build_predicted, AnomalyContext, AnomalySignal, ContextFrame,
    GapContext, GapSignal, SessionHistory, StagedIntent,
};

fn main() {
    println!("=== hydra-context test harness ===\n");
    let mut passed = 0_u32;
    let failed = 0_u32;

    // --- Active window ---
    let input = make_input("deploy the api service now", 0.8);
    let active = build_active(&input);
    passed += check("active: window from comprehended input", !active.is_empty());
    passed += check(
        "active: contains domain signal",
        active
            .items
            .iter()
            .any(|i| i.content.contains("domain signal")),
    );

    // --- Historical window ---
    let mut history = SessionHistory::new();
    history.add(make_input("first input deploy api", 0.5));
    history.add(make_input("second input check budget", 0.3));
    let hist_window = build_historical(&history);
    passed += check("historical: window from session", hist_window.len() == 2);
    passed += check(
        "historical: by_domain filters",
        history.by_domain("engineering").len() == 2,
    );

    // --- Predicted window ---
    let staged = vec![
        StagedIntent::new("low priority", 0.3, "session"),
        StagedIntent::new("high priority", 0.9, "temporal"),
        StagedIntent::new("medium priority", 0.6, "task"),
    ];
    let pred_window = build_predicted(&staged);
    passed += check(
        "predicted: ordering by confidence",
        pred_window.items[0].significance >= pred_window.items[1].significance,
    );

    // --- Gap threshold ---
    let mut gaps = GapContext::new();
    gaps.add_gap(GapSignal::new("weak gap", 0.3));
    passed += check("gap: rejects below threshold", gaps.is_empty());
    gaps.add_gap(GapSignal::new("strong gap", 0.7));
    passed += check("gap: accepts above threshold", gaps.len() == 1);

    // --- Anomaly threshold ---
    let mut anomalies = AnomalyContext::new();
    anomalies.add_anomaly(AnomalySignal::new("weak anomaly", 0.4));
    passed += check("anomaly: rejects below threshold", anomalies.is_empty());
    anomalies.add_anomaly(AnomalySignal::new("strong anomaly", 0.8));
    passed += check("anomaly: accepts above threshold", anomalies.len() == 1);

    // --- Full frame ---
    let frame = ContextFrame::build(&input, &history, &staged, &gaps, &anomalies);
    passed += check(
        "frame: all 5 windows present",
        !frame.active.is_empty()
            && !frame.historical.is_empty()
            && !frame.predicted.is_empty()
            && !frame.gaps.is_empty()
            && !frame.anomalies.is_empty(),
    );

    // --- Summary format ---
    let summary = frame.summary();
    passed += check(
        "frame: summary format",
        summary.contains("active=") && summary.contains("gaps="),
    );

    // --- Final banner ---
    let total = passed + failed;
    println!("\n========================================");
    println!("  hydra-context: {passed}/{total} passed, {failed} failed");
    if failed == 0 {
        println!("  ALL TESTS PASSED");
    } else {
        println!("  SOME TESTS FAILED");
    }
    println!("========================================");

    if failed > 0 {
        std::process::exit(1);
    }
}

fn make_input(raw: &str, urgency: f64) -> ComprehendedInput {
    ComprehendedInput {
        raw: raw.to_string(),
        primary_domain: Domain::Engineering,
        all_domains: vec![(Domain::Engineering, 0.6)],
        primitives: vec![],
        temporal: TemporalContext {
            urgency,
            horizon: Horizon::ShortTerm,
            constraint_status: ConstraintStatus::None,
        },
        resonance: ResonanceResult::empty(),
        source: InputSource::PrincipalText,
        confidence: 0.7,
        used_llm: false,
    }
}

fn check(name: &str, ok: bool) -> u32 {
    if ok {
        println!("  PASS: {name}");
        1
    } else {
        println!("  FAIL: {name}");
        0
    }
}
