//! Deductive reasoning — axiom-based logical inference.
//!
//! Zero LLM. Maps axiom primitives to logical conclusions
//! via pattern matching on primitive combinations.

use crate::conclusion::{ReasoningConclusion, ReasoningMode};
use crate::constants::DEDUCTIVE_MIN_CONFIDENCE;
use hydra_attention::AttentionFrame;
use hydra_axiom::AxiomPrimitive;
use hydra_comprehension::ComprehendedInput;

/// Run deductive reasoning on comprehended input and attention frame.
///
/// Checks axiom primitive combinations to derive logical conclusions.
/// Returns `None` if no deductive conclusion can be drawn.
/// Zero LLM calls — pure axiom-based inference.
pub fn run_deductive(
    input: &ComprehendedInput,
    attention: &AttentionFrame,
) -> Option<ReasoningConclusion> {
    let primitives = &input.primitives;
    if primitives.is_empty() {
        return None;
    }

    let has = |p: &AxiomPrimitive| primitives.contains(p);
    let focus_count = attention.focus_items.len();
    let base_confidence = compute_base_confidence(primitives.len(), focus_count);

    if base_confidence < DEDUCTIVE_MIN_CONFIDENCE {
        return None;
    }

    // Risk + CausalLink → causally connected risk
    if has(&AxiomPrimitive::Risk) && has(&AxiomPrimitive::CausalLink) {
        return Some(build_conclusion(
            "Elements are causally connected with identified risk propagation paths",
            base_confidence + 0.1,
            vec![
                "Risk primitive detected".into(),
                "CausalLink primitive detected".into(),
                format!("{} focus items reinforce connection", focus_count),
            ],
        ));
    }

    // Dependency + Risk → dependency risk
    if has(&AxiomPrimitive::Dependency) && has(&AxiomPrimitive::Risk) {
        return Some(build_conclusion(
            "Dependency chain carries risk — failure may propagate",
            base_confidence + 0.05,
            vec![
                "Dependency primitive detected".into(),
                "Risk primitive detected".into(),
            ],
        ));
    }

    // TrustRelation + Risk → trust boundary risk
    if has(&AxiomPrimitive::TrustRelation) && has(&AxiomPrimitive::Risk) {
        return Some(build_conclusion(
            "Trust boundary intersects risk zone — verify trust assumptions",
            base_confidence + 0.05,
            vec![
                "TrustRelation primitive detected".into(),
                "Risk primitive detected".into(),
            ],
        ));
    }

    // Risk alone → risk conditions exist
    if has(&AxiomPrimitive::Risk) {
        return Some(build_conclusion(
            "Risk conditions exist in the input domain",
            base_confidence,
            vec!["Risk primitive detected".into()],
        ));
    }

    // CausalLink alone → causal structure detected
    if has(&AxiomPrimitive::CausalLink) {
        return Some(build_conclusion(
            "Causal structure detected — effects follow from identified causes",
            base_confidence,
            vec!["CausalLink primitive detected".into()],
        ));
    }

    // Constraint + Optimization → constrained optimization
    if has(&AxiomPrimitive::Constraint) && has(&AxiomPrimitive::Optimization) {
        return Some(build_conclusion(
            "Constrained optimization problem — solution must satisfy constraints",
            base_confidence,
            vec![
                "Constraint primitive detected".into(),
                "Optimization primitive detected".into(),
            ],
        ));
    }

    None
}

/// Compute base confidence from primitive count and focus count.
fn compute_base_confidence(prim_count: usize, focus_count: usize) -> f64 {
    let prim_factor = (prim_count as f64 * 0.15).min(0.6);
    let focus_factor = (focus_count as f64 * 0.05).min(0.2);
    (prim_factor + focus_factor + 0.2).min(1.0)
}

/// Build a deductive conclusion with the given parameters.
fn build_conclusion(
    statement: &str,
    confidence: f64,
    supporting: Vec<String>,
) -> ReasoningConclusion {
    ReasoningConclusion::new(
        ReasoningMode::Deductive,
        statement,
        confidence,
        supporting,
        false, // Zero LLM
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty_frame() -> AttentionFrame {
        let budget = hydra_attention::budget::AttentionBudget::compute(
            &hydra_language::IntentKind::StatusQuery,
            &hydra_language::AffectSignal {
                register: hydra_language::InteractionRegister::Neutral,
                confidence: 0.7,
                keywords_detected: vec![],
            },
        );
        AttentionFrame {
            focus_items: vec![],
            summary_items: vec![],
            filtered_count: 0,
            budget,
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
    fn risk_and_causal_link() {
        let input = make_input(vec![AxiomPrimitive::Risk, AxiomPrimitive::CausalLink]);
        let result = run_deductive(&input, &empty_frame());
        assert!(result.is_some());
        let c = result.unwrap();
        assert!(c.statement.contains("causally connected"));
        assert!(!c.used_llm);
    }

    #[test]
    fn empty_primitives_returns_none() {
        let input = make_input(vec![]);
        assert!(run_deductive(&input, &empty_frame()).is_none());
    }

    #[test]
    fn risk_alone() {
        let input = make_input(vec![AxiomPrimitive::Risk]);
        let result = run_deductive(&input, &empty_frame());
        assert!(result.is_some());
        assert!(result.unwrap().statement.contains("conditions exist"));
    }
}
