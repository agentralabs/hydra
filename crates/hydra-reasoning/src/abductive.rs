//! Abductive reasoning — best-explanation inference from observations.
//!
//! Builds observations from attention focus items and axiom primitives.
//! Uses axiom-based explanations when possible (zero LLM).
//! Flags `used_llm = true` only when primitive count < 2 AND
//! confidence is below threshold (future LLM integration point).

use crate::conclusion::{ReasoningConclusion, ReasoningMode};
use crate::constants::ABDUCTIVE_LLM_THRESHOLD;
use hydra_attention::AttentionFrame;
use hydra_axiom::AxiomPrimitive;
use hydra_comprehension::ComprehendedInput;

/// Run abductive reasoning on comprehended input and attention frame.
///
/// Builds observations from focus items and primitives, then selects
/// the best explanation. Uses axiom-based reasoning when primitives
/// are available. Zero LLM when primitives >= 2 or confidence >= threshold.
pub fn run_abductive(
    input: &ComprehendedInput,
    attention: &AttentionFrame,
) -> Option<ReasoningConclusion> {
    let primitives = &input.primitives;
    let focus_items = &attention.focus_items;

    // Need at least some signal to reason about.
    if primitives.is_empty() && focus_items.is_empty() {
        return None;
    }

    let observations = build_observations(primitives, focus_items.len());
    if observations.is_empty() {
        return None;
    }

    let prim_count = primitives.len();
    let confidence = compute_abductive_confidence(prim_count, focus_items.len());

    // Axiom-based explanation when primitives exist.
    if prim_count >= 2 {
        let explanation = build_multi_primitive_explanation(primitives);
        return Some(ReasoningConclusion::new(
            ReasoningMode::Abductive,
            explanation,
            confidence,
            observations,
            false, // Zero LLM — axiom mapping succeeded
        ));
    }

    // Single primitive or focus-only — still axiom-based if we have primitives.
    if prim_count == 1 {
        let explanation = build_single_primitive_explanation(&primitives[0]);
        let needs_llm = confidence < ABDUCTIVE_LLM_THRESHOLD;
        return Some(ReasoningConclusion::new(
            ReasoningMode::Abductive,
            explanation,
            confidence,
            observations,
            needs_llm,
        ));
    }

    // No primitives, only focus items — flag LLM need.
    Some(ReasoningConclusion::new(
        ReasoningMode::Abductive,
        "Observations present but no axiom primitives for explanation — LLM recommended",
        confidence,
        observations,
        true, // Needs LLM
    ))
}

/// Build observation strings from primitives and focus items.
fn build_observations(primitives: &[AxiomPrimitive], focus_count: usize) -> Vec<String> {
    let mut obs = Vec::new();
    for p in primitives {
        obs.push(format!("Observed primitive: {}", p.label()));
    }
    if focus_count > 0 {
        obs.push(format!("{} items in attention focus", focus_count));
    }
    obs
}

/// Compute abductive confidence from available signals.
fn compute_abductive_confidence(prim_count: usize, focus_count: usize) -> f64 {
    let prim_factor = (prim_count as f64 * 0.2).min(0.6);
    let focus_factor = (focus_count as f64 * 0.05).min(0.2);
    (prim_factor + focus_factor + 0.1).min(1.0)
}

/// Build explanation from multiple primitives.
fn build_multi_primitive_explanation(primitives: &[AxiomPrimitive]) -> String {
    let labels: Vec<&str> = primitives.iter().map(|p| p.label()).collect();
    format!(
        "Best explanation: the co-occurrence of [{}] suggests a systemic pattern \
         requiring coordinated attention",
        labels.join(", "),
    )
}

/// Build explanation from a single primitive.
fn build_single_primitive_explanation(primitive: &AxiomPrimitive) -> String {
    format!(
        "Best explanation: {} is the primary driver of observed conditions",
        primitive.label(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_attention::scorer::ScoredItem;

    fn make_budget() -> hydra_attention::budget::AttentionBudget {
        hydra_attention::budget::AttentionBudget::compute(
            &hydra_language::IntentKind::StatusQuery,
            &hydra_language::AffectSignal {
                register: hydra_language::InteractionRegister::Neutral,
                confidence: 0.7,
                keywords_detected: vec![],
            },
        )
    }

    fn empty_frame() -> AttentionFrame {
        AttentionFrame {
            focus_items: vec![],
            summary_items: vec![],
            filtered_count: 0,
            budget: make_budget(),
        }
    }

    fn frame_with_focus(n: usize) -> AttentionFrame {
        let items: Vec<ScoredItem> = (0..n)
            .map(|i| ScoredItem {
                content: format!("focus-{}", i),
                base_score: 0.8,
                final_score: 0.8,
                bonuses: vec![],
                domain: None,
            })
            .collect();
        AttentionFrame {
            focus_items: items,
            summary_items: vec![],
            filtered_count: 0,
            budget: make_budget(),
        }
    }

    fn make_input(primitives: Vec<AxiomPrimitive>) -> ComprehendedInput {
        use hydra_comprehension::resonance::ResonanceResult;
        use hydra_comprehension::temporal::{ConstraintStatus, Horizon, TemporalContext};
        use hydra_comprehension::{Domain, InputSource};
        ComprehendedInput {
            raw: "test".into(),
            primary_domain: Domain::Engineering,
            all_domains: vec![(Domain::Engineering, 0.5)],
            primitives,
            temporal: TemporalContext {
                urgency: 0.5,
                horizon: Horizon::Immediate,
                constraint_status: ConstraintStatus::None,
            },
            resonance: ResonanceResult::empty(),
            source: InputSource::PrincipalText,
            confidence: 0.7,
            used_llm: false,
        }
    }

    #[test]
    fn multi_primitive_zero_llm() {
        let input = make_input(vec![AxiomPrimitive::Risk, AxiomPrimitive::CausalLink]);
        let result = run_abductive(&input, &empty_frame());
        assert!(result.is_some());
        let c = result.unwrap();
        assert!(!c.used_llm);
        assert!(c.statement.contains("co-occurrence"));
    }

    #[test]
    fn empty_returns_none() {
        let input = make_input(vec![]);
        assert!(run_abductive(&input, &empty_frame()).is_none());
    }

    #[test]
    fn focus_only_flags_llm() {
        let input = make_input(vec![]);
        let frame = frame_with_focus(3);
        let result = run_abductive(&input, &frame);
        assert!(result.is_some());
        assert!(result.unwrap().used_llm);
    }
}
