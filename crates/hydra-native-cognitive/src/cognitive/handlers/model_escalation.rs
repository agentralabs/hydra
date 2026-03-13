//! Model escalation — detect low-quality responses and escalate to stronger models.
//!
//! Phase 4 of the superintelligence plan. Starts with the cheapest model that
//! might work, detects quality issues, and escalates to stronger models when needed.

use crate::cognitive::intent_router::{IntentCategory, ClassifiedIntent};

/// Decision about whether to escalate to a stronger model.
#[derive(Debug, Clone)]
pub(crate) struct EscalationDecision {
    pub should_escalate: bool,
    pub reason: &'static str,
    pub target_model: String,
}

/// Check if the LLM response quality warrants escalation to a stronger model.
///
/// Returns Some(decision) if escalation is recommended.
pub(crate) fn check_escalation(
    response: &str,
    intent: &ClassifiedIntent,
    complexity: &str,
    current_model: &str,
) -> Option<EscalationDecision> {
    // Already at max model — can't escalate further
    let next = next_model(current_model)?;

    // Never escalate greetings or simple intents
    if matches!(intent.category, IntentCategory::Greeting | IntentCategory::Farewell) {
        return None;
    }

    let response_len = response.len();

    // Check for "I don't know" / uncertainty markers
    let uncertainty_phrases = [
        "i'm not sure", "i don't know", "i cannot determine",
        "i'm unable to", "i can't help with", "beyond my ability",
        "i apologize but i cannot",
    ];
    let response_lower = response.to_lowercase();
    let has_uncertainty = uncertainty_phrases.iter().any(|p| response_lower.contains(p));

    if has_uncertainty && complexity == "complex" {
        return Some(EscalationDecision {
            should_escalate: true,
            reason: "uncertainty_on_complex",
            target_model: next,
        });
    }

    // Check for placeholder code (before short-response, as placeholders are more specific)
    let placeholder_markers = ["todo!()", "unimplemented!()", "// TODO", "/* TODO */",
        "pass  # TODO", "raise NotImplementedError"];
    let has_placeholders = placeholder_markers.iter().any(|p| response.contains(p));
    if has_placeholders && complexity == "complex" {
        return Some(EscalationDecision {
            should_escalate: true,
            reason: "placeholder_code",
            target_model: next,
        });
    }

    // Check for suspiciously short responses on complex code tasks
    if matches!(intent.category, IntentCategory::CodeBuild | IntentCategory::CodeFix) {
        if complexity == "complex" && response_len < 200 {
            return Some(EscalationDecision {
                should_escalate: true,
                reason: "short_code_response",
                target_model: next,
            });
        }
    }

    // Check for refusal to act
    if response_lower.contains("i can't") && response_lower.contains("instead") && complexity == "complex" {
        return Some(EscalationDecision {
            should_escalate: true,
            reason: "refusal_with_redirect",
            target_model: next,
        });
    }

    None
}

/// Get the next model in the escalation chain.
fn next_model(current: &str) -> Option<String> {
    if current.contains("haiku") {
        Some("claude-sonnet-4-6".into())
    } else if current.contains("sonnet") {
        Some("claude-opus-4-6".into())
    } else if current.contains("gpt-4o-mini") {
        Some("gpt-4o".into())
    } else {
        None // Already at max
    }
}

/// Select the initial model for an intent category based on complexity.
/// This is the starting point before any escalation.
pub(crate) fn select_initial_model(
    intent: &ClassifiedIntent,
    complexity: &str,
    category_success_rate: Option<f64>,
) -> &'static str {
    // If historical success rate with cheap model is low, start higher
    if let Some(rate) = category_success_rate {
        if rate < 0.5 && complexity == "complex" {
            return "claude-sonnet-4-6";
        }
    }

    match intent.category {
        // Simple intents → always Haiku
        IntentCategory::Greeting | IntentCategory::Farewell
        | IntentCategory::Thanks => "claude-haiku-4-5-20251001",

        // Code tasks: complex → Sonnet, simple → Haiku
        IntentCategory::CodeBuild | IntentCategory::CodeFix => {
            if complexity == "complex" { "claude-sonnet-4-6" }
            else { "claude-haiku-4-5-20251001" }
        }

        // Deploy: always Sonnet (safety critical)
        IntentCategory::Deploy => "claude-sonnet-4-6",

        // Self-implement: always Sonnet
        IntentCategory::SelfImplement => "claude-sonnet-4-6",

        // Everything else: based on complexity
        _ => {
            if complexity == "complex" { "claude-sonnet-4-6" }
            else { "claude-haiku-4-5-20251001" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_intent(cat: IntentCategory) -> ClassifiedIntent {
        ClassifiedIntent { category: cat, confidence: 0.9, target: None, payload: None }
    }

    #[test]
    fn test_no_escalation_greeting() {
        let result = check_escalation(
            "Hello! How can I help?",
            &mock_intent(IntentCategory::Greeting),
            "simple",
            "claude-haiku-4-5-20251001",
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_escalation_on_uncertainty() {
        let result = check_escalation(
            "I'm not sure how to implement that correctly.",
            &mock_intent(IntentCategory::CodeBuild),
            "complex",
            "claude-haiku-4-5-20251001",
        );
        assert!(result.is_some());
        let d = result.unwrap();
        assert!(d.target_model.contains("sonnet"));
        assert_eq!(d.reason, "uncertainty_on_complex");
    }

    #[test]
    fn test_escalation_short_code_response() {
        let result = check_escalation(
            "Here is the function: fn foo() {}",
            &mock_intent(IntentCategory::CodeBuild),
            "complex",
            "claude-haiku-4-5-20251001",
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().reason, "short_code_response");
    }

    #[test]
    fn test_escalation_placeholder_code() {
        let result = check_escalation(
            "```rust\nfn process() {\n    todo!()\n}\n```",
            &mock_intent(IntentCategory::CodeBuild),
            "complex",
            "claude-haiku-4-5-20251001",
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().reason, "placeholder_code");
    }

    #[test]
    fn test_no_escalation_at_max() {
        let result = check_escalation(
            "I'm not sure.",
            &mock_intent(IntentCategory::CodeBuild),
            "complex",
            "claude-opus-4-6",
        );
        assert!(result.is_none()); // Already at max model
    }

    #[test]
    fn test_next_model_chain() {
        assert_eq!(next_model("claude-haiku-4-5-20251001"), Some("claude-sonnet-4-6".into()));
        assert_eq!(next_model("claude-sonnet-4-6"), Some("claude-opus-4-6".into()));
        assert_eq!(next_model("claude-opus-4-6"), None);
    }

    #[test]
    fn test_select_initial_model_simple() {
        let m = select_initial_model(&mock_intent(IntentCategory::Greeting), "simple", None);
        assert!(m.contains("haiku"));
    }

    #[test]
    fn test_select_initial_model_complex_code() {
        let m = select_initial_model(&mock_intent(IntentCategory::CodeBuild), "complex", None);
        assert!(m.contains("sonnet"));
    }

    #[test]
    fn test_select_initial_with_low_success_rate() {
        let m = select_initial_model(&mock_intent(IntentCategory::CodeExplain), "complex", Some(0.3));
        assert!(m.contains("sonnet")); // Upgraded due to low success rate
    }
}
