//! Intent extraction — classifies comprehended input by keyword matching.
//!
//! Zero LLM calls. Pure structural classification.

use hydra_comprehension::ComprehendedInput;
use serde::{Deserialize, Serialize};

/// The kind of intent detected in the input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentKind {
    /// User wants an action performed (deploy, build, execute).
    ActionRequest,
    /// User wants analysis or explanation.
    AnalysisRequest,
    /// User wants verification or validation.
    VerificationRequest,
    /// User wants planning or architecture help.
    PlanningAssist,
    /// User wants content generation.
    GenerativeRequest,
    /// Conversational acknowledgement (thanks, ok).
    Conversational,
    /// User is asking about current status.
    StatusQuery,
    /// General information request (default).
    InformationRequest,
}

impl IntentKind {
    /// Return a human-readable label for this intent kind.
    pub fn label(&self) -> &str {
        match self {
            Self::ActionRequest => "action-request",
            Self::AnalysisRequest => "analysis-request",
            Self::VerificationRequest => "verification-request",
            Self::PlanningAssist => "planning-assist",
            Self::GenerativeRequest => "generative-request",
            Self::Conversational => "conversational",
            Self::StatusQuery => "status-query",
            Self::InformationRequest => "information-request",
        }
    }
}

/// The result of intent extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    /// The classified intent kind.
    pub kind: IntentKind,
    /// Confidence in this classification (0.0 to 1.0).
    pub confidence: f64,
    /// The action verb that triggered this classification, if any.
    pub action_verb: Option<String>,
}

/// Action keywords for each intent kind.
const ACTION_KEYWORDS: &[&str] = &["deploy", "build", "execute", "run", "start"];
const ANALYSIS_KEYWORDS: &[&str] = &["why", "how", "analyze", "explain", "cause"];
const VERIFICATION_KEYWORDS: &[&str] = &["check", "verify", "validate", "confirm"];
const PLANNING_KEYWORDS: &[&str] = &["plan", "design", "architect", "strategy"];
const GENERATIVE_KEYWORDS: &[&str] = &["create", "generate", "write"];
const CONVERSATIONAL_KEYWORDS: &[&str] = &["thanks", "got it", "ok", "sure"];
const STATUS_KEYWORDS: &[&str] = &["status", "what is", "show me"];

/// Extract intent from a comprehended input. Zero LLM calls.
///
/// Classifies by keyword matching against the raw input text.
/// Returns the best-matching intent with confidence and action verb.
pub fn extract_intent(input: &ComprehendedInput) -> IntentResult {
    let lower = input.raw.to_lowercase();

    // Check each category in priority order.
    // Conversational and status checked early (strong, specific signals).
    // Analysis before action to avoid "build" stealing "why did the build fail".
    if let Some(verb) = find_keyword(&lower, CONVERSATIONAL_KEYWORDS) {
        return make_extraction(IntentKind::Conversational, 0.9, Some(verb));
    }

    if let Some(verb) = find_keyword(&lower, STATUS_KEYWORDS) {
        return make_extraction(IntentKind::StatusQuery, 0.8, Some(verb));
    }

    if let Some(verb) = find_keyword(&lower, ANALYSIS_KEYWORDS) {
        return make_extraction(IntentKind::AnalysisRequest, 0.8, Some(verb));
    }

    if let Some(verb) = find_keyword(&lower, VERIFICATION_KEYWORDS) {
        return make_extraction(IntentKind::VerificationRequest, 0.8, Some(verb));
    }

    if let Some(verb) = find_keyword(&lower, PLANNING_KEYWORDS) {
        return make_extraction(IntentKind::PlanningAssist, 0.75, Some(verb));
    }

    if let Some(verb) = find_keyword(&lower, GENERATIVE_KEYWORDS) {
        return make_extraction(IntentKind::GenerativeRequest, 0.75, Some(verb));
    }

    if let Some(verb) = find_keyword(&lower, ACTION_KEYWORDS) {
        // Disambiguate: "build" can be generative if content-focused
        if verb == "build" && is_content_focused(&lower) {
            return make_extraction(IntentKind::GenerativeRequest, 0.75, Some(verb));
        }
        return make_extraction(IntentKind::ActionRequest, 0.8, Some(verb));
    }

    // Default: information request
    make_extraction(IntentKind::InformationRequest, 0.5, None)
}

/// Find the first matching keyword in the text using word-boundary-aware matching.
///
/// Multi-word phrases (containing spaces) use substring matching.
/// Single words require word boundary matching to avoid partial matches.
fn find_keyword(text: &str, keywords: &[&str]) -> Option<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    keywords
        .iter()
        .find(|kw| {
            if kw.contains(' ') {
                // Multi-word phrase: substring match is fine
                text.contains(**kw)
            } else {
                // Single word: must match a whole word
                words
                    .iter()
                    .any(|w| w.trim_matches(|c: char| !c.is_alphanumeric()) == **kw)
            }
        })
        .map(|kw| kw.to_string())
}

/// Check whether text is content-focused (for generative disambiguation).
fn is_content_focused(text: &str) -> bool {
    let content_signals = ["document", "report", "template", "page", "article"];
    content_signals.iter().any(|s| text.contains(s))
}

/// Construct an IntentResult.
fn make_extraction(
    kind: IntentKind,
    confidence: f64,
    action_verb: Option<String>,
) -> IntentResult {
    IntentResult {
        kind,
        confidence,
        action_verb,
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
    fn classifies_action() {
        let e = extract_intent(&make_input("deploy the service now"));
        assert_eq!(e.kind, IntentKind::ActionRequest);
    }

    #[test]
    fn classifies_analysis() {
        let e = extract_intent(&make_input("why did the build fail"));
        assert_eq!(e.kind, IntentKind::AnalysisRequest);
    }

    #[test]
    fn classifies_verification() {
        let e = extract_intent(&make_input("verify the ssl certificate"));
        assert_eq!(e.kind, IntentKind::VerificationRequest);
    }

    #[test]
    fn classifies_conversational() {
        let e = extract_intent(&make_input("thanks for the help"));
        assert_eq!(e.kind, IntentKind::Conversational);
    }

    #[test]
    fn classifies_status() {
        let e = extract_intent(&make_input("show me the current status"));
        assert_eq!(e.kind, IntentKind::StatusQuery);
    }

    #[test]
    fn defaults_to_information() {
        let e = extract_intent(&make_input("something entirely random here"));
        assert_eq!(e.kind, IntentKind::InformationRequest);
    }
}
