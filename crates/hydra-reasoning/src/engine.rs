//! Reasoning engine — runs all five modes and synthesizes results.

use crate::abductive::run_abductive;
use crate::adversarial::run_adversarial;
use crate::analogical::run_analogical;
use crate::conclusion::{ReasoningConclusion, ReasoningMode};
use crate::constants::{
    MAX_CONCLUSIONS, SYNTHESIS_CONFIDENCE_THRESHOLD, WEIGHT_ABDUCTIVE, WEIGHT_ADVERSARIAL,
    WEIGHT_ANALOGICAL, WEIGHT_DEDUCTIVE, WEIGHT_INDUCTIVE,
};
use crate::deductive::run_deductive;
use crate::errors::ReasoningError;
use crate::inductive::run_inductive;
use hydra_attention::AttentionFrame;
use hydra_comprehension::ComprehendedInput;
use hydra_genome::GenomeStore;

/// The synthesized result of running all five reasoning modes.
#[derive(Debug, Clone)]
pub struct ReasoningResult {
    /// All conclusions, sorted by confidence descending (max `MAX_CONCLUSIONS`).
    pub conclusions: Vec<ReasoningConclusion>,
    /// Weighted average confidence across active modes.
    pub synthesis_confidence: f64,
    /// Whether any mode used (or flagged the need for) an LLM.
    pub used_llm: bool,
    /// Number of modes that produced at least one conclusion.
    pub active_modes: usize,
    /// The highest-confidence conclusion, if any.
    pub primary: Option<ReasoningConclusion>,
    /// Summary of each mode: (label, produced_conclusion).
    pub mode_summary: Vec<(String, bool)>,
}

impl ReasoningResult {
    /// Return a TUI-friendly summary of the reasoning result.
    pub fn summary(&self) -> String {
        let primary_text = self
            .primary
            .as_ref()
            .map(|c| c.statement.clone())
            .unwrap_or_else(|| "none".into());

        let modes: Vec<String> = self
            .mode_summary
            .iter()
            .map(|(name, active)| {
                let marker = if *active { "+" } else { "-" };
                format!("{}{}", marker, name)
            })
            .collect();

        let llm_tag = if self.used_llm { "LLM" } else { "pure" };

        format!(
            "reasoning[{}]: synthesis={:.2} modes=[{}] active={}/{} primary=\"{}\"",
            llm_tag,
            self.synthesis_confidence,
            modes.join(" "),
            self.active_modes,
            self.mode_summary.len(),
            primary_text,
        )
    }
}

/// The core reasoning engine that orchestrates all five modes.
pub struct ReasoningEngine;

impl ReasoningEngine {
    /// Create a new reasoning engine.
    pub fn new() -> Self {
        Self
    }

    /// Run all five reasoning modes and synthesize a result.
    ///
    /// Each mode runs independently. Conclusions are collected,
    /// sorted by confidence, and capped at `MAX_CONCLUSIONS`.
    /// Synthesis confidence is a weighted average of active modes.
    pub fn reason(
        &self,
        input: &ComprehendedInput,
        attention: &AttentionFrame,
        genome: &GenomeStore,
    ) -> Result<ReasoningResult, ReasoningError> {
        if attention.focus_items.is_empty() && attention.summary_items.is_empty() {
            // Allow reasoning if we have primitives even with empty frame.
            if input.primitives.is_empty() {
                return Err(ReasoningError::EmptyAttentionFrame);
            }
        }

        let deductive = run_deductive(input, attention);
        let inductive = run_inductive(input, attention, genome);
        let abductive = run_abductive(input, attention);
        let analogical = run_analogical(input, attention, genome);
        let adversarial = run_adversarial(input, attention);

        let mode_results: Vec<(ReasoningMode, Option<ReasoningConclusion>)> = vec![
            (ReasoningMode::Deductive, deductive),
            (ReasoningMode::Inductive, inductive),
            (ReasoningMode::Abductive, abductive),
            (ReasoningMode::Analogical, analogical),
            (ReasoningMode::Adversarial, adversarial),
        ];

        let mode_summary: Vec<(String, bool)> = mode_results
            .iter()
            .map(|(mode, result)| (mode.label().to_string(), result.is_some()))
            .collect();

        let mut conclusions: Vec<ReasoningConclusion> = mode_results
            .into_iter()
            .filter_map(|(_, result)| result)
            .collect();

        if conclusions.is_empty() {
            return Err(ReasoningError::NoConclusions);
        }

        // Sort by confidence descending.
        conclusions.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        conclusions.truncate(MAX_CONCLUSIONS);

        let used_llm = conclusions.iter().any(|c| c.used_llm);
        let active_modes = mode_summary.iter().filter(|(_, active)| *active).count();
        let synthesis_confidence = compute_synthesis_confidence(&conclusions);
        let primary = conclusions.first().cloned();

        if synthesis_confidence < SYNTHESIS_CONFIDENCE_THRESHOLD {
            return Err(ReasoningError::LowSynthesisConfidence {
                confidence: synthesis_confidence,
                threshold: SYNTHESIS_CONFIDENCE_THRESHOLD,
            });
        }

        Ok(ReasoningResult {
            conclusions,
            synthesis_confidence,
            used_llm,
            active_modes,
            primary,
            mode_summary,
        })
    }
}

impl Default for ReasoningEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute weighted average synthesis confidence from conclusions.
fn compute_synthesis_confidence(conclusions: &[ReasoningConclusion]) -> f64 {
    if conclusions.is_empty() {
        return 0.0;
    }

    let mut weighted_sum = 0.0;
    let mut weight_sum = 0.0;

    for c in conclusions {
        let weight = mode_weight(&c.mode);
        weighted_sum += c.confidence * weight;
        weight_sum += weight;
    }

    if weight_sum == 0.0 {
        return 0.0;
    }

    weighted_sum / weight_sum
}

/// Return the weight for a given reasoning mode.
fn mode_weight(mode: &ReasoningMode) -> f64 {
    match mode {
        ReasoningMode::Deductive => WEIGHT_DEDUCTIVE,
        ReasoningMode::Inductive => WEIGHT_INDUCTIVE,
        ReasoningMode::Abductive => WEIGHT_ABDUCTIVE,
        ReasoningMode::Analogical => WEIGHT_ANALOGICAL,
        ReasoningMode::Adversarial => WEIGHT_ADVERSARIAL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_axiom::AxiomPrimitive;

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
            raw: "test reasoning input".into(),
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
    fn engine_produces_deductive() {
        let engine = ReasoningEngine::new();
        let input = make_input(vec![AxiomPrimitive::Risk, AxiomPrimitive::CausalLink]);
        let genome = GenomeStore::new();
        let result = engine.reason(&input, &empty_frame(), &genome);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.active_modes >= 1);
        assert!(!r.conclusions.is_empty());
    }

    #[test]
    fn empty_input_fails() {
        let engine = ReasoningEngine::new();
        let input = make_input(vec![]);
        let genome = GenomeStore::new();
        let result = engine.reason(&input, &empty_frame(), &genome);
        assert!(result.is_err());
    }

    #[test]
    fn synthesis_confidence_computed() {
        let c = ReasoningConclusion::new(ReasoningMode::Deductive, "test", 0.8, vec![], false);
        let conf = compute_synthesis_confidence(&[c]);
        assert!(conf > 0.0);
    }

    #[test]
    fn summary_format() {
        let engine = ReasoningEngine::new();
        let input = make_input(vec![AxiomPrimitive::Risk, AxiomPrimitive::CausalLink]);
        let genome = GenomeStore::new();
        let result = engine.reason(&input, &empty_frame(), &genome).unwrap();
        let s = result.summary();
        assert!(s.contains("reasoning"));
        assert!(s.contains("synthesis="));
        assert!(s.contains("active="));
    }
}
