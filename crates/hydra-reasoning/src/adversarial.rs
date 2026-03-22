//! Adversarial reasoning — threat modeling and attack surface analysis.
//!
//! Zero LLM. Only fires when security-related primitives are present
//! (Risk + TrustRelation, or Security domain).

use crate::conclusion::{ReasoningConclusion, ReasoningMode};
use crate::constants::ADVERSARIAL_MIN_THREAT_CONFIDENCE;
use hydra_attention::AttentionFrame;
use hydra_axiom::AxiomPrimitive;
use hydra_comprehension::{ComprehendedInput, Domain};

/// Run adversarial reasoning on comprehended input.
///
/// Only activates when security-relevant signals are present:
/// - Risk + TrustRelation primitives, OR
/// - Security domain classification.
///
/// Returns `None` for non-security contexts.
/// Zero LLM calls — pure axiom-based threat modeling.
pub fn run_adversarial(
    input: &ComprehendedInput,
    attention: &AttentionFrame,
) -> Option<ReasoningConclusion> {
    let primitives = &input.primitives;
    let is_security_domain = input.primary_domain == Domain::Security;
    let has_risk = primitives.contains(&AxiomPrimitive::Risk);
    let has_trust = primitives.contains(&AxiomPrimitive::TrustRelation);
    let has_adversarial = primitives.contains(&AxiomPrimitive::AdversarialModel);

    // Only fire for security-relevant contexts.
    let should_activate = (has_risk && has_trust) || is_security_domain || has_adversarial;

    if !should_activate {
        return None;
    }

    let confidence =
        compute_threat_confidence(primitives, is_security_domain, attention.focus_items.len());

    if confidence < ADVERSARIAL_MIN_THREAT_CONFIDENCE {
        return None;
    }

    let threat_surfaces = identify_threat_surfaces(primitives, is_security_domain);
    let mut supporting = vec![
        format!("security domain: {}", is_security_domain),
        format!(
            "threat-relevant primitives: {}",
            count_threat_primitives(primitives)
        ),
    ];
    for surface in &threat_surfaces {
        supporting.push(format!("threat surface: {}", surface));
    }
    if !attention.focus_items.is_empty() {
        supporting.push(format!(
            "{} items in attention focus",
            attention.focus_items.len()
        ));
    }

    Some(ReasoningConclusion::new(
        ReasoningMode::Adversarial,
        format!(
            "Threat model: {} attack surface(s) identified — {}",
            threat_surfaces.len(),
            threat_surfaces.join(", "),
        ),
        confidence,
        supporting,
        false, // Zero LLM
    ))
}

/// Compute threat confidence from security signals.
fn compute_threat_confidence(
    primitives: &[AxiomPrimitive],
    is_security_domain: bool,
    focus_count: usize,
) -> f64 {
    let mut confidence = 0.2;

    if is_security_domain {
        confidence += 0.2;
    }

    let threat_count = count_threat_primitives(primitives);
    confidence += threat_count as f64 * 0.15;
    confidence += (focus_count as f64 * 0.03).min(0.15);

    confidence.min(1.0)
}

/// Count primitives relevant to threat modeling.
fn count_threat_primitives(primitives: &[AxiomPrimitive]) -> usize {
    primitives
        .iter()
        .filter(|p| {
            matches!(
                p,
                AxiomPrimitive::Risk
                    | AxiomPrimitive::TrustRelation
                    | AxiomPrimitive::AdversarialModel
                    | AxiomPrimitive::Uncertainty
            )
        })
        .count()
}

/// Identify threat surfaces from primitives and domain.
fn identify_threat_surfaces(
    primitives: &[AxiomPrimitive],
    is_security_domain: bool,
) -> Vec<String> {
    let mut surfaces = Vec::new();

    if primitives.contains(&AxiomPrimitive::TrustRelation) {
        surfaces.push("trust boundary violation".into());
    }
    if primitives.contains(&AxiomPrimitive::Risk) {
        surfaces.push("risk exposure".into());
    }
    if primitives.contains(&AxiomPrimitive::AdversarialModel) {
        surfaces.push("adversarial vector".into());
    }
    if is_security_domain && surfaces.is_empty() {
        surfaces.push("general security concern".into());
    }

    surfaces
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

    fn make_input(domain: Domain, primitives: Vec<AxiomPrimitive>) -> ComprehendedInput {
        use hydra_comprehension::resonance::ResonanceResult;
        use hydra_comprehension::temporal::{ConstraintStatus, Horizon, TemporalContext};
        use hydra_comprehension::InputSource;
        ComprehendedInput {
            raw: "test security input".into(),
            primary_domain: domain,
            all_domains: vec![(Domain::Security, 0.8)],
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
    fn fires_on_risk_and_trust() {
        let input = make_input(
            Domain::Engineering,
            vec![AxiomPrimitive::Risk, AxiomPrimitive::TrustRelation],
        );
        let result = run_adversarial(&input, &empty_frame());
        assert!(result.is_some());
        let c = result.unwrap();
        assert!(!c.used_llm);
        assert!(c.statement.contains("attack surface"));
    }

    #[test]
    fn fires_on_security_domain() {
        let input = make_input(Domain::Security, vec![AxiomPrimitive::Risk]);
        let result = run_adversarial(&input, &empty_frame());
        assert!(result.is_some());
    }

    #[test]
    fn skips_finance_no_security_primitives() {
        let input = make_input(Domain::Finance, vec![AxiomPrimitive::Optimization]);
        assert!(run_adversarial(&input, &empty_frame()).is_none());
    }
}
