//! Inductive reasoning — pattern generalization from genome history.
//!
//! Zero LLM. Queries the genome store for similar past situations
//! and generalizes from observed patterns.

use crate::conclusion::{ReasoningConclusion, ReasoningMode};
use crate::constants::INDUCTIVE_MIN_OCCURRENCES;
use hydra_attention::AttentionFrame;
use hydra_comprehension::ComprehendedInput;
use hydra_genome::{GenomeStore, SituationSignature};

/// Trait extension for extracting a situation signature from input.
pub trait SituationSignatureExt {
    /// Derive a situation signature from this input.
    fn situation_signature(&self) -> SituationSignature;
}

impl SituationSignatureExt for ComprehendedInput {
    /// Derive a situation signature by extracting keywords from the raw input.
    fn situation_signature(&self) -> SituationSignature {
        SituationSignature::from_description(&self.raw)
    }
}

/// Run inductive reasoning using genome history.
///
/// Queries the genome for similar past situations. Requires at least
/// `INDUCTIVE_MIN_OCCURRENCES` matches to generalize.
/// Returns `None` if insufficient history exists.
/// Zero LLM calls — pure genome pattern matching.
pub fn run_inductive(
    input: &ComprehendedInput,
    _attention: &AttentionFrame,
    genome: &GenomeStore,
) -> Option<ReasoningConclusion> {
    let matches = genome.query(&input.raw);

    if matches.len() < INDUCTIVE_MIN_OCCURRENCES {
        return None;
    }

    let total_confidence: f64 = matches.iter().map(|e| e.effective_confidence()).sum();
    let avg_confidence = total_confidence / matches.len() as f64;

    let best = matches.iter().max_by(|a, b| {
        a.effective_confidence()
            .partial_cmp(&b.effective_confidence())
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;

    let best_approach = &best.approach.approach_type;
    let match_count = matches.len();

    let mut supporting = vec![
        format!("{} similar situations found in genome", match_count),
        format!("average confidence: {:.2}", avg_confidence),
        format!("best approach type: {}", best_approach),
    ];

    if best.use_count > 0 {
        supporting.push(format!(
            "best approach used {} times with {} successes",
            best.use_count, best.success_count,
        ));
    }

    Some(ReasoningConclusion::new(
        ReasoningMode::Inductive,
        format!(
            "Pattern from {} similar situations suggests '{}' approach \
             with {:.0}% historical confidence",
            match_count,
            best_approach,
            avg_confidence * 100.0,
        ),
        avg_confidence,
        supporting,
        false, // Zero LLM
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_attention::AttentionFrame;
    use hydra_genome::signature::ApproachSignature;

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

    fn make_input(raw: &str) -> ComprehendedInput {
        use hydra_comprehension::resonance::ResonanceResult;
        use hydra_comprehension::temporal::{ConstraintStatus, Horizon, TemporalContext};
        use hydra_comprehension::{Domain, InputSource};
        ComprehendedInput {
            raw: raw.into(),
            primary_domain: Domain::Engineering,
            all_domains: vec![(Domain::Engineering, 0.5)],
            primitives: vec![],
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

    fn make_approach() -> ApproachSignature {
        ApproachSignature::new("deploy", vec!["build".into()], vec!["docker".into()])
    }

    #[test]
    fn inductive_needs_min_occurrences() {
        let mut genome = GenomeStore::new();
        genome
            .add_from_operation("deploy rest api service", make_approach(), 0.8)
            .unwrap();

        let input = make_input("deploy rest api service");
        let result = run_inductive(&input, &empty_frame(), &genome);
        // Only 1 match, need at least 2
        assert!(result.is_none());
    }

    #[test]
    fn inductive_with_enough_history() {
        let mut genome = GenomeStore::new();
        genome
            .add_from_operation("deploy rest api service", make_approach(), 0.8)
            .unwrap();
        genome
            .add_from_operation("deploy rest api service now", make_approach(), 0.7)
            .unwrap();

        let input = make_input("deploy rest api service");
        let result = run_inductive(&input, &empty_frame(), &genome);
        assert!(result.is_some());
        let c = result.unwrap();
        assert!(c.statement.contains("deploy"));
        assert!(!c.used_llm);
    }

    #[test]
    fn situation_signature_trait() {
        let input = make_input("deploy the rest api service");
        let sig = input.situation_signature();
        assert!(!sig.keywords.is_empty());
    }
}
