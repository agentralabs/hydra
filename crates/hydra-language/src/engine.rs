//! Language engine — combines intent, hedge, depth, and affect analysis.
//!
//! Zero LLM calls. The engine is the single entry point for language analysis.

use crate::affect::{detect_affect, AffectSignal, InteractionRegister};
use crate::depth::{detect_depth, DepthLevel};
use crate::errors::LanguageError;
use crate::hedge::{detect_hedges, HedgeResult};
use crate::intent::{extract_intent, IntentResult, IntentKind};
use hydra_comprehension::ComprehendedInput;
use serde::{Deserialize, Serialize};

/// Recommended response depth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseDepth {
    /// 1–2 sentences.
    Brief,
    /// Standard paragraph response.
    Standard,
    /// Full analysis, multiple sections.
    Deep,
}

impl ResponseDepth {
    /// Return a human-readable label.
    pub fn label(&self) -> &str {
        match self {
            Self::Brief => "brief",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }
}

/// The complete result of language analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageAnalysis {
    /// Intent extraction result.
    pub intent: IntentResult,
    /// Hedge detection result.
    pub hedge: HedgeResult,
    /// Depth detection result.
    pub depth: DepthLevel,
    /// Affect detection result.
    pub affect: AffectSignal,
    /// Overall confidence (intent confidence minus hedge penalty).
    pub confidence: f64,
    /// Human-readable framing summary.
    pub framing: String,
    /// Recommended response depth.
    pub response_depth: ResponseDepth,
}

/// The language analysis engine. Stateless.
pub struct LanguageEngine;

impl LanguageEngine {
    /// Analyze a comprehended input for intent, hedge, depth, and affect.
    ///
    /// Zero LLM calls. Returns a complete `LanguageAnalysis`.
    pub fn analyze(input: &ComprehendedInput) -> Result<LanguageAnalysis, LanguageError> {
        if input.raw.trim().is_empty() {
            return Err(LanguageError::EmptyInput);
        }

        let intent = extract_intent(input);
        let hedge = detect_hedges(&input.raw);
        let depth = detect_depth(&input.raw);
        let affect = detect_affect(&input.raw);

        // Compute confidence: intent confidence minus hedge penalty, clamped.
        let confidence = (intent.confidence - hedge.penalty).clamp(0.0, 1.0);

        // Determine response depth.
        let response_depth = compute_response_depth(&intent, &affect);

        // Build framing summary.
        let framing = build_framing(&intent, &hedge, &depth, &affect);

        Ok(LanguageAnalysis {
            intent,
            hedge,
            depth,
            affect,
            confidence,
            framing,
            response_depth,
        })
    }
}

/// Compute the recommended response depth.
///
/// Crisis always overrides to Brief.
fn compute_response_depth(intent: &IntentResult, affect: &AffectSignal) -> ResponseDepth {
    // Crisis override: always Brief.
    if affect.register == InteractionRegister::Crisis {
        return ResponseDepth::Brief;
    }

    match intent.kind {
        IntentKind::Conversational => ResponseDepth::Brief,
        IntentKind::StatusQuery => ResponseDepth::Standard,
        IntentKind::ActionRequest => ResponseDepth::Standard,
        IntentKind::VerificationRequest => ResponseDepth::Standard,
        IntentKind::AnalysisRequest => ResponseDepth::Deep,
        IntentKind::PlanningAssist => ResponseDepth::Deep,
        IntentKind::GenerativeRequest => ResponseDepth::Deep,
        IntentKind::InformationRequest => ResponseDepth::Standard,
    }
}

/// Build a human-readable framing summary.
fn build_framing(
    intent: &IntentResult,
    hedge: &HedgeResult,
    depth: &DepthLevel,
    affect: &AffectSignal,
) -> String {
    let mut parts = vec![format!("intent={}", intent.kind.label())];

    if hedge.is_hedged {
        parts.push(format!("hedged({})", hedge.hedge_words.join(",")));
    }

    parts.push(format!("depth={}", depth.label()));
    parts.push(format!("affect={}", affect.register.label()));

    parts.join(" ")
}

impl Default for LanguageEngine {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_comprehension::{
        ConstraintStatus, Domain, Horizon, InputSource, ResonanceResult, TemporalContext,
    };

    fn make_input(raw: &str) -> ComprehendedInput {
        ComprehendedInput {
            raw: raw.to_string(),
            primary_domain: Domain::Engineering,
            all_domains: vec![(Domain::Engineering, 0.5)],
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
    fn crisis_overrides_to_brief() {
        let r = LanguageEngine::analyze(&make_input("the site is broken and down, users affected"))
            .expect("should succeed");
        assert_eq!(r.response_depth, ResponseDepth::Brief);
    }

    #[test]
    fn hedging_reduces_confidence() {
        let certain =
            LanguageEngine::analyze(&make_input("deploy the service now")).expect("should succeed");
        let hedged =
            LanguageEngine::analyze(&make_input("maybe we should probably deploy the service"))
                .expect("should succeed");
        assert!(hedged.confidence < certain.confidence);
    }

    #[test]
    fn empty_input_rejected() {
        let input = ComprehendedInput {
            raw: "".to_string(),
            primary_domain: Domain::Unknown,
            all_domains: vec![],
            primitives: vec![],
            temporal: TemporalContext {
                urgency: 0.0,
                horizon: Horizon::LongTerm,
                constraint_status: ConstraintStatus::None,
            },
            resonance: ResonanceResult::empty(),
            source: InputSource::PrincipalText,
            confidence: 0.0,
            used_llm: false,
        };
        assert!(LanguageEngine::analyze(&input).is_err());
    }

    #[test]
    fn framing_contains_intent() {
        let r =
            LanguageEngine::analyze(&make_input("deploy the service now")).expect("should succeed");
        assert!(r.framing.contains("intent="));
    }
}
