//! Intelligence wiring — connects OutcomeTracker, CalibrationTracker, and
//! self-improvement checks to the cognitive loop. Includes DB persistence
//! for cross-session intelligence.
//!
//! Called from loop_runner.rs between phases. Phases 3, 6, 7 of the
//! superintelligence plan.

use std::sync::Arc;
use tokio::sync::mpsc;

use super::super::intent_router::{ClassifiedIntent, IntentCategory};
use super::super::loop_runner::CognitiveUpdate;
use super::super::metacognition::CalibrationTracker;
use super::super::outcome_tracker::{Outcome, OutcomeTracker};

/// Load intelligence state from DB into the trackers.
///
/// Populates OutcomeTracker category stats and CalibrationTracker buckets
/// from persisted data, so intelligence accumulates across sessions.
pub(crate) fn load_from_db(
    tracker: &mut OutcomeTracker,
    calibration: &mut CalibrationTracker,
    db: &hydra_db::HydraDb,
) {
    // Load calibration buckets
    if let Ok(buckets) = db.load_calibration_buckets() {
        let has_data = buckets.iter().any(|(t, _)| *t > 0);
        if has_data {
            calibration.load_buckets(buckets);
            let total: u64 = buckets.iter().map(|(t, _)| t).sum();
            eprintln!("[hydra:intelligence] Loaded {} calibration predictions from DB", total);
        }
    }

    // Load recent outcomes and replay into tracker
    if let Ok(outcomes) = db.load_outcomes(500) {
        let count = outcomes.len();
        for row in outcomes.into_iter().rev() {
            // oldest first
            let category = IntentCategory::from_str(&row.intent_category);
            let outcome = match row.outcome.as_str() {
                "success" => Outcome::Success,
                "correction" => Outcome::Correction,
                "failure" => Outcome::Failure,
                "repeat" => Outcome::Repeat,
                _ => Outcome::Neutral,
            };
            tracker.record(category, &row.topic, &row.model_used, outcome, row.tokens_used as u64);
        }
        if count > 0 {
            eprintln!("[hydra:intelligence] Loaded {} outcomes from DB", count);
        }
    }
}

/// Save intelligence state to DB after processing.
pub(crate) fn save_to_db(
    intent: &ClassifiedIntent,
    topic: &str,
    model: &str,
    outcome: &Outcome,
    tokens: u64,
    calibration: &CalibrationTracker,
    db: &hydra_db::HydraDb,
) {
    // Save this outcome
    let _ = db.save_outcome(
        intent.category.as_db_str(),
        topic,
        model,
        outcome.as_db_str(),
        tokens,
    );

    // Save calibration buckets
    let _ = db.save_calibration_buckets(calibration.buckets());
}

/// Populate the outcome tracker from conversation history (session-local).
///
/// This is a fallback when no DB is available. When DB is available,
/// `load_from_db` provides richer cross-session data.
pub(crate) fn populate_from_history(
    tracker: &mut OutcomeTracker,
    calibration: &mut CalibrationTracker,
    history: &[(String, String)],
) {
    if history.len() < 3 {
        return;
    }

    for window in history.windows(2) {
        let (prev_role, prev_content) = &window[0];
        let (curr_role, curr_content) = &window[1];

        if prev_role == "hydra" || prev_role == "assistant" {
            if curr_role == "user" || curr_role == "human" {
                let outcome = tracker.detect_outcome(prev_content, curr_content, &[]);
                let topic = extract_topic(curr_content);
                tracker.record(IntentCategory::Question, &topic, "unknown", outcome.clone(), 0);
                let success = matches!(outcome, Outcome::Success);
                calibration.record(0.7, success);
            }
        }
    }

    if tracker.total_interactions() > 0 {
        eprintln!(
            "[hydra:intelligence] Populated {} outcomes from history",
            tracker.total_interactions()
        );
    }
}

/// Run metacognitive assessment and send insight to UI.
pub(crate) fn assess_and_report(
    intent: &ClassifiedIntent,
    complexity: &str,
    tracker: &OutcomeTracker,
    calibration: &CalibrationTracker,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let cat_rate = tracker.category_success_rate(intent.category);
    let cal_error = calibration.calibration_error();

    let assessment = super::super::metacognition::assess_interaction(
        intent, complexity, cat_rate, cal_error,
    );

    let msg = format!(
        "confidence={:?} cal_err={:.2} cat_success={:.0}% verify={} caveats={}",
        assessment.confidence_level,
        cal_error,
        cat_rate * 100.0,
        assessment.should_verify,
        assessment.should_add_caveats,
    );

    let _ = tx.send(CognitiveUpdate::MetacognitiveInsight { assessment: msg });
    eprintln!(
        "[hydra:metacognition] {:?} cal_err={:.2} cat_rate={:.2}",
        assessment.confidence_level, cal_error, cat_rate,
    );
}

/// Run self-improvement check if enough data has accumulated.
pub(crate) fn check_self_improvement(
    tracker: &OutcomeTracker,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let total = tracker.total_interactions();
    if total < 10 { return; }
    if total % 20 != 0 { return; }

    let weak = tracker.weak_categories(0.5);
    if weak.is_empty() { return; }

    let stats: Vec<(String, f64, u64)> = weak
        .iter()
        .map(|(cat, rate)| (cat.clone(), *rate, 20u64))
        .collect();

    let candidates = hydra_kernel::self_improve::identify_weaknesses(&stats, 10, 0.6);
    for c in &candidates {
        let _spec = hydra_kernel::self_improve::generate_improvement_spec(c);
        let _ = tx.send(CognitiveUpdate::MetacognitiveInsight {
            assessment: format!(
                "Self-improvement: {} ({:.0}% success over {} interactions)",
                c.weakness, c.success_rate * 100.0, c.sample_count,
            ),
        });
        eprintln!("[hydra:self-improve] Candidate: {} → {:?}", c.category, c.suggested_fix);
    }
}

/// Extract a simple topic key from user text.
fn extract_topic(text: &str) -> String {
    text.split_whitespace()
        .filter(|w| w.len() > 3)
        .take(3)
        .collect::<Vec<_>>()
        .join("_")
        .to_lowercase()
}
