//! Handlers for sister improvement (P10) and threat queries (P11).

use tokio::sync::mpsc;
use crate::cognitive::intent_router::{ClassifiedIntent, IntentCategory};
use crate::cognitive::loop_runner::CognitiveUpdate;

/// Handle "improve the X sister" intent.
pub(crate) async fn handle_sister_improve(
    text: &str,
    intent: &ClassifiedIntent,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if intent.category != IntentCategory::SisterImprove {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Sister Improvement".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    // Extract path and goal from user text
    let full_text = text.to_string();
    let sister_path = match crate::sister_improve::extract_sister_path(&full_text) {
        Some(p) => p,
        None => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: "I need a path to the sister project to improve it.\n\n\
                    Usage: `improve sister at ../agentic-memory add retry logic`\n\
                    Or: `/improve-sister ../agentic-memory --auto`".into(),
                css_class: "message hydra".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            return true;
        }
    };

    let goal = crate::sister_improve::extract_goal(&full_text);

    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: format!(
            "Starting improvement on `{}`\nGoal: **{}**",
            sister_path.display(), goal
        ),
        css_class: "message hydra".into(),
    });

    // Run the improvement pipeline
    let (improve_tx, mut improve_rx) = mpsc::channel(100);
    let path = sister_path.clone();
    let goal_clone = goal.clone();

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let improver = crate::sister_improve::SisterImprover::new();
        let report = improver.improve(&path, &goal_clone, &improve_tx).await;
        let _ = tx_clone.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: format!("**Result:** {}", report.summary()),
            css_class: "message hydra".into(),
        });
        let _ = tx_clone.send(CognitiveUpdate::ResetIdle);
    });

    // Drain immediate progress updates
    while let Ok(update) = improve_rx.try_recv() {
        if let CognitiveUpdate::Phase(msg) = update {
            let _ = tx.send(CognitiveUpdate::Phase(msg));
        }
    }

    true
}

/// Handle "what's the threat level?" intent.
pub(crate) fn handle_threat_query(
    _text: &str,
    intent: &ClassifiedIntent,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if intent.category != IntentCategory::ThreatQuery {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Threat Intelligence".into()));

    let correlator = crate::threat::ThreatCorrelator::new();
    let summary = correlator.summary();
    let patterns = correlator.patterns_summary();

    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: format!("{}\n\n{}", summary, patterns),
        css_class: "message hydra".into(),
    });
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}
