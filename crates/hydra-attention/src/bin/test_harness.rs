//! Test harness for hydra-attention.
//!
//! Runs ~15 scenarios covering budget computation, allocation,
//! frame properties, and the full integration pipeline.

use hydra_attention::budget::{AttentionBudget, ProcessingDepth};
use hydra_attention::constants::{FULL_DEPTH_COST, SUMMARY_COST};
use hydra_attention::engine::AttentionEngine;
use hydra_attention::frame::AttentionFrame;
use hydra_attention::{allocate, score_all_items, ScoredItem};
use hydra_comprehension::{
    ComprehendedInput, ConstraintStatus, Domain, Horizon, InputSource, ResonanceResult,
    TemporalContext,
};
use hydra_context::{
    AnomalyContext, AnomalySignal, ContextFrame, ContextItem, ContextWindow, GapContext, GapSignal,
    SessionHistory, StagedIntent,
};
use hydra_language::{AffectSignal, IntentKind, InteractionRegister, LanguageEngine};

fn main() {
    let mut passed = 0;
    let mut failed = 0;

    macro_rules! check {
        ($name:expr, $cond:expr) => {
            if $cond {
                println!("  PASS: {}", $name);
                passed += 1;
            } else {
                println!("  FAIL: {}", $name);
                failed += 1;
            }
        };
    }

    println!("=== hydra-attention test harness ===\n");

    // --- Budget computation ---
    println!("[budget computation]");

    let neutral = AffectSignal {
        register: InteractionRegister::Neutral,
        confidence: 0.7,
        keywords_detected: vec![],
    };

    let analysis_budget = AttentionBudget::compute(&IntentKind::AnalysisRequest, &neutral);
    let status_budget = AttentionBudget::compute(&IntentKind::StatusQuery, &neutral);
    check!(
        "analysis > status budget",
        analysis_budget.total() > status_budget.total()
    );

    let crisis_affect = AffectSignal {
        register: InteractionRegister::Crisis,
        confidence: 0.9,
        keywords_detected: vec!["broken".into()],
    };
    let exploratory_affect = AffectSignal {
        register: InteractionRegister::Exploratory,
        confidence: 0.7,
        keywords_detected: vec![],
    };

    let crisis_budget = AttentionBudget::compute(&IntentKind::ActionRequest, &crisis_affect);
    let neutral_budget = AttentionBudget::compute(&IntentKind::ActionRequest, &neutral);
    let exploratory_budget =
        AttentionBudget::compute(&IntentKind::ActionRequest, &exploratory_affect);
    check!(
        "crisis < neutral < exploratory",
        crisis_budget.total() < neutral_budget.total()
            && neutral_budget.total() < exploratory_budget.total()
    );

    let mut budget = AttentionBudget::compute(&IntentKind::AnalysisRequest, &neutral);
    let initial_remaining = budget.remaining();
    let _ = budget.consume(ProcessingDepth::Full);
    check!(
        "consumption tracking",
        budget.remaining() == initial_remaining - FULL_DEPTH_COST
    );

    let _ = budget.consume(ProcessingDepth::Summary);
    check!(
        "summary consumption",
        budget.remaining() == initial_remaining - FULL_DEPTH_COST - SUMMARY_COST
    );

    // --- Allocation ---
    println!("\n[allocation]");

    let high_item = ScoredItem {
        content: "high".into(),
        base_score: 0.8,
        final_score: 0.8,
        bonuses: vec![],
        domain: None,
    };
    let mid_item = ScoredItem {
        content: "mid".into(),
        base_score: 0.3,
        final_score: 0.3,
        bonuses: vec![],
        domain: None,
    };
    let low_item = ScoredItem {
        content: "low".into(),
        base_score: 0.05,
        final_score: 0.05,
        bonuses: vec![],
        domain: None,
    };

    let mut alloc_budget = AttentionBudget::compute(&IntentKind::AnalysisRequest, &neutral);
    let allocated = allocate(&[high_item.clone(), mid_item, low_item], &mut alloc_budget);
    check!(
        "analysis budget allocates full+summary+filtered",
        allocated.len() == 3
            && allocated[0].depth == ProcessingDepth::Full
            && allocated[1].depth == ProcessingDepth::Summary
            && allocated[2].depth == ProcessingDepth::Filtered
    );

    // Crisis narrows focus.
    let mut crisis_alloc_budget =
        AttentionBudget::compute(&IntentKind::ActionRequest, &crisis_affect);
    let crisis_items: Vec<ScoredItem> = (0..10)
        .map(|i| ScoredItem {
            content: format!("item-{i}"),
            base_score: 0.7,
            final_score: 0.7,
            bonuses: vec![],
            domain: None,
        })
        .collect();
    let crisis_allocated = allocate(&crisis_items, &mut crisis_alloc_budget);
    let _crisis_full_count = crisis_allocated
        .iter()
        .filter(|a| a.depth == ProcessingDepth::Full)
        .count();
    let neutral_full_budget = AttentionBudget::compute(&IntentKind::ActionRequest, &neutral);
    check!(
        "crisis narrows focus",
        crisis_alloc_budget.total() < neutral_full_budget.total()
    );

    // Anomaly gets attention.
    let mut anomaly_window = ContextWindow::new("anomalies");
    anomaly_window.add(ContextItem::new("anomaly detected", 0.8));
    let empty_window = ContextWindow::new("empty");
    let scored_with_anomaly = score_all_items(
        &empty_window,
        &empty_window,
        &empty_window,
        &empty_window,
        &anomaly_window,
        0.3,
        false,
        None,
    );
    check!(
        "anomaly gets attention boost",
        scored_with_anomaly[0].final_score > 0.8
    );

    // Historical items scored.
    let mut hist_window = ContextWindow::new("historical");
    hist_window.add(ContextItem::new("past event", 0.5));
    let scored_with_hist = score_all_items(
        &empty_window,
        &hist_window,
        &empty_window,
        &empty_window,
        &empty_window,
        0.3,
        false,
        None,
    );
    check!(
        "historical items attended",
        !scored_with_hist.is_empty() && scored_with_hist[0].base_score == 0.5
    );

    // --- Frame properties ---
    println!("\n[frame properties]");

    let frame_budget = AttentionBudget::compute(&IntentKind::StatusQuery, &neutral);
    let frame = AttentionFrame::from_allocated(vec![], frame_budget);
    let summary = frame.summary();
    check!(
        "summary format contains keywords",
        summary.contains("attention:")
            && summary.contains("focus=")
            && summary.contains("utilization=")
    );

    let frame_budget2 = AttentionBudget::compute(&IntentKind::AnalysisRequest, &neutral);
    let frame2 = AttentionFrame::from_allocated(vec![], frame_budget2);
    check!(
        "utilization bounds [0, 1]",
        frame2.utilization() >= 0.0 && frame2.utilization() <= 1.0
    );

    // --- Integration ---
    println!("\n[integration]");

    let input = make_input("analyze the system architecture deeply");
    let language = LanguageEngine::analyze(&input).expect("language analysis");

    let mut history = SessionHistory::new();
    history.add(make_input("deploy the service"));
    let staged = vec![StagedIntent::new("review code", 0.6, "session")];
    let mut gaps = GapContext::new();
    gaps.add_gap(GapSignal::new("missing metrics", 0.7));
    let mut anomalies = AnomalyContext::new();
    anomalies.add_anomaly(AnomalySignal::new("unusual latency", 0.8));

    let context = ContextFrame::build(&input, &history, &staged, &gaps, &anomalies);
    let result = AttentionEngine::allocate(&input, &context, &language);
    check!("full pipeline succeeds", result.is_ok());

    let attention = result.expect("pipeline result");
    check!(
        "pipeline produces attended items",
        attention.attended_count() > 0
    );
    check!(
        "pipeline utilization in bounds",
        attention.utilization() >= 0.0 && attention.utilization() <= 1.0
    );
    check!("pipeline has focus", attention.has_focus());

    // --- Summary ---
    println!("\n=== Results: {} passed, {} failed ===", passed, failed);
    if failed > 0 {
        std::process::exit(1);
    }
}

/// Helper to create a ComprehendedInput for testing.
fn make_input(raw: &str) -> ComprehendedInput {
    ComprehendedInput {
        raw: raw.to_string(),
        primary_domain: Domain::Engineering,
        all_domains: vec![(Domain::Engineering, 0.6)],
        primitives: vec![],
        temporal: TemporalContext {
            urgency: 0.5,
            horizon: Horizon::ShortTerm,
            constraint_status: ConstraintStatus::None,
        },
        resonance: ResonanceResult::empty(),
        source: InputSource::PrincipalText,
        confidence: 0.7,
        used_llm: false,
    }
}
