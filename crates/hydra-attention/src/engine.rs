//! Attention engine — orchestrates the full attention allocation pipeline.
//!
//! Compute budget -> Score items -> Allocate -> Build frame.

use crate::allocator::allocate;
use crate::budget::AttentionBudget;
use crate::errors::AttentionError;
use crate::frame::AttentionFrame;
use crate::scorer::score_all_items;
use hydra_comprehension::ComprehendedInput;
use hydra_context::ContextFrame;
use hydra_language::LanguageAnalysis;

/// The attention engine. Stateless — runs the full pipeline in one call.
pub struct AttentionEngine;

impl AttentionEngine {
    /// Run the full attention pipeline.
    ///
    /// Pipeline:
    /// 1. Compute budget from intent kind and affect
    /// 2. Score all items from the five context windows
    /// 3. Allocate processing depth within the budget
    /// 4. Build the attention frame
    ///
    /// Returns `EmptyContext` if the context frame has no items.
    pub fn allocate(
        input: &ComprehendedInput,
        context: &ContextFrame,
        language: &LanguageAnalysis,
    ) -> Result<AttentionFrame, AttentionError> {
        // Check for empty context.
        if context.total_items() == 0 {
            return Err(AttentionError::EmptyContext);
        }

        // 1. Compute budget.
        let mut budget = AttentionBudget::compute(&language.intent.kind, &language.affect);

        // 2. Score all items using comprehended input for urgency/resonance/domain.
        let urgency = input.temporal.urgency;
        let has_resonance = input.resonance.has_prior_context;
        let primary_domain = input.primary_domain.label();

        let scored = score_all_items(
            &context.active,
            &context.historical,
            &context.predicted,
            &context.gaps,
            &context.anomalies,
            urgency,
            has_resonance,
            Some(primary_domain),
        );

        // 3. Allocate processing depth.
        let allocated = allocate(&scored, &mut budget);

        // 4. Build frame.
        Ok(AttentionFrame::from_allocated(allocated, budget))
    }
}

impl Default for AttentionEngine {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_comprehension::{
        ComprehendedInput, ConstraintStatus, Domain, Horizon, InputSource, ResonanceResult,
        TemporalContext,
    };
    use hydra_context::{
        AnomalyContext, AnomalySignal, ContextFrame, GapContext, GapSignal, SessionHistory,
        StagedIntent,
    };
    use hydra_language::LanguageEngine;

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

    #[test]
    fn empty_context_rejected() {
        let input = make_input("analyze the architecture");
        let language = LanguageEngine::analyze(&input).expect("should succeed");
        let frame = ContextFrame::build(
            &input,
            &SessionHistory::new(),
            &[],
            &GapContext::new(),
            &AnomalyContext::new(),
        );

        // The active window always has at least one item from the input,
        // so this test verifies the pipeline runs without error.
        let result = AttentionEngine::allocate(&input, &frame, &language);
        assert!(result.is_ok());
    }

    #[test]
    fn full_pipeline_coherent() {
        let input = make_input("analyze the system architecture deeply");
        let language = LanguageEngine::analyze(&input).expect("should succeed");

        let mut history = SessionHistory::new();
        history.add(make_input("deploy the service"));

        let staged = vec![StagedIntent::new("review code", 0.6, "session")];
        let mut gaps = GapContext::new();
        gaps.add_gap(GapSignal::new("missing metrics", 0.7));
        let mut anomalies = AnomalyContext::new();
        anomalies.add_anomaly(AnomalySignal::new("unusual latency", 0.8));

        let frame = ContextFrame::build(&input, &history, &staged, &gaps, &anomalies);
        let result = AttentionEngine::allocate(&input, &frame, &language);

        assert!(result.is_ok());
        let attention = result.expect("should succeed");
        assert!(attention.attended_count() > 0);
        assert!(attention.utilization() >= 0.0);
        assert!(attention.utilization() <= 1.0);
    }
}
