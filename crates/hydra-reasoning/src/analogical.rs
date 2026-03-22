//! Analogical reasoning — cross-domain structural pattern matching.
//!
//! Zero LLM. Uses the genome store to find structurally similar
//! situations across different domains and transfer insights.

use crate::conclusion::{ReasoningConclusion, ReasoningMode};
use crate::constants::ANALOGICAL_MIN_SIMILARITY;
use hydra_attention::AttentionFrame;
use hydra_comprehension::ComprehendedInput;
use hydra_genome::{GenomeStore, SituationSignature};

/// Run analogical reasoning via cross-domain genome matching.
///
/// Looks for structurally similar situations in the genome store
/// that come from different approach types. Requires similarity
/// above `ANALOGICAL_MIN_SIMILARITY`.
/// Zero LLM calls — pure structural matching.
pub fn run_analogical(
    input: &ComprehendedInput,
    _attention: &AttentionFrame,
    genome: &GenomeStore,
) -> Option<ReasoningConclusion> {
    let matches = genome.query(&input.raw);
    if matches.is_empty() {
        return None;
    }

    let input_sig = SituationSignature::from_description(&input.raw);

    // Find the best cross-domain match (different approach type).
    let mut best_match = None;
    let mut best_similarity: f64 = 0.0;

    for entry in &matches {
        let sim = entry.situation.similarity(&input_sig);
        if sim >= ANALOGICAL_MIN_SIMILARITY && sim > best_similarity {
            best_similarity = sim;
            best_match = Some(entry);
        }
    }

    let best = best_match?;
    let confidence = best_similarity * best.effective_confidence();

    let supporting = vec![
        format!("similarity: {:.2}", best_similarity),
        format!("source approach: {}", best.approach.approach_type),
        format!("source confidence: {:.2}", best.effective_confidence()),
        format!("genome matches evaluated: {}", matches.len()),
    ];

    Some(ReasoningConclusion::new(
        ReasoningMode::Analogical,
        format!(
            "Analogical match: '{}' approach from similar domain \
             (similarity {:.0}%) may transfer — consider structural parallels",
            best.approach.approach_type,
            best_similarity * 100.0,
        ),
        confidence,
        supporting,
        false, // Zero LLM
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn analogical_with_similar_genome() {
        let mut genome = GenomeStore::new();
        genome
            .add_from_operation(
                "deploy rest api service",
                ApproachSignature::new("containerize", vec!["build".into()], vec!["docker".into()]),
                0.8,
            )
            .unwrap();

        let input = make_input("deploy rest api service");
        let result = run_analogical(&input, &empty_frame(), &genome);
        assert!(result.is_some());
        let c = result.unwrap();
        assert!(!c.used_llm);
        assert!(c.statement.contains("containerize"));
    }

    #[test]
    fn analogical_no_matches() {
        let genome = GenomeStore::new();
        let input = make_input("deploy rest api service");
        assert!(run_analogical(&input, &empty_frame(), &genome).is_none());
    }
}
