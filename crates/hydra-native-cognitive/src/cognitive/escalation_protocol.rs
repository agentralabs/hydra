//! Escalation protocol — structured multi-level failure recovery.
//!
//! UCU Module #12 (Wave 2). Extends existing model_escalation.rs with richer
//! strategies: prompt refinement, decomposition, and human-in-the-loop.
//! Why not a sister? Decision logic is purely in-memory pattern matching.

use crate::cognitive::context_manager::ModelTier;
use crate::cognitive::intent_router::{ClassifiedIntent, IntentCategory};

/// Escalation severity levels — ordered from least to most disruptive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EscalationLevel {
    /// Response is acceptable — no action needed.
    None,
    /// Same model, refined prompt with more context/specificity.
    RetryPrompt,
    /// Switch to a more capable model.
    UpgradeModel,
    /// Task is too complex for single-shot — decompose it.
    DecomposeTask,
    /// Cannot solve autonomously — ask the user.
    HumanReview,
}

/// Full escalation decision with actionable details.
#[derive(Debug, Clone)]
pub struct EscalationProtocol {
    pub level: EscalationLevel,
    pub target_model: Option<String>,
    pub reason: String,
    /// Hint for how to refine the prompt on retry.
    pub retry_prompt_hint: Option<String>,
    /// Suggested subtask decomposition (if DecomposeTask).
    pub subtasks: Vec<String>,
}

impl EscalationProtocol {
    pub fn none() -> Self {
        Self {
            level: EscalationLevel::None,
            target_model: None,
            reason: String::new(),
            retry_prompt_hint: None,
            subtasks: Vec::new(),
        }
    }
}

/// Determine the escalation strategy based on response quality signals.
pub fn escalate(
    response: &str,
    intent: &ClassifiedIntent,
    complexity: &str,
    current_model: &str,
    attempt: u8,
) -> EscalationProtocol {
    // Never escalate social interactions
    if matches!(intent.category, IntentCategory::Greeting | IntentCategory::Farewell | IntentCategory::Thanks) {
        return EscalationProtocol::none();
    }

    let response_lower = response.to_lowercase();
    let quality = assess_response_quality(response, &response_lower, intent, complexity);

    // Quality is acceptable
    if quality >= 0.7 {
        return EscalationProtocol::none();
    }

    // Escalation strategy depends on attempt number
    match attempt {
        0 | 1 => {
            // First failure: try refining the prompt
            if let Some(hint) = suggest_prompt_refinement(response, &response_lower, intent) {
                return EscalationProtocol {
                    level: EscalationLevel::RetryPrompt,
                    target_model: None,
                    reason: "low_quality_first_attempt".into(),
                    retry_prompt_hint: Some(hint),
                    subtasks: Vec::new(),
                };
            }
            // No prompt fix possible — upgrade model
            if let Some(next) = next_model(current_model) {
                EscalationProtocol {
                    level: EscalationLevel::UpgradeModel,
                    target_model: Some(next),
                    reason: format!("quality={:.2}_attempt_{}", quality, attempt),
                    retry_prompt_hint: None,
                    subtasks: Vec::new(),
                }
            } else {
                EscalationProtocol::none() // Already at max, accept what we have
            }
        }
        2 => {
            // Second failure: upgrade model if not already at max
            if let Some(next) = next_model(current_model) {
                EscalationProtocol {
                    level: EscalationLevel::UpgradeModel,
                    target_model: Some(next),
                    reason: format!("quality={:.2}_second_failure", quality),
                    retry_prompt_hint: Some("Provide a more detailed, step-by-step response.".into()),
                    subtasks: Vec::new(),
                }
            } else if complexity == "complex" {
                // At max model, complex task — try decomposition
                EscalationProtocol {
                    level: EscalationLevel::DecomposeTask,
                    target_model: None,
                    reason: "max_model_complex_task".into(),
                    retry_prompt_hint: None,
                    subtasks: suggest_decomposition(intent),
                }
            } else {
                EscalationProtocol::none()
            }
        }
        _ => {
            // 3+ failures — escalate to human
            EscalationProtocol {
                level: EscalationLevel::HumanReview,
                target_model: None,
                reason: format!("exhausted_retries_attempt_{}", attempt),
                retry_prompt_hint: None,
                subtasks: Vec::new(),
            }
        }
    }
}

/// Assess response quality on a 0.0-1.0 scale.
fn assess_response_quality(
    response: &str,
    lower: &str,
    intent: &ClassifiedIntent,
    complexity: &str,
) -> f32 {
    let mut score: f32 = 1.0;

    // Uncertainty markers
    let uncertainty = ["i'm not sure", "i don't know", "i cannot determine",
        "i'm unable to", "i can't help with", "beyond my ability"];
    if uncertainty.iter().any(|p| lower.contains(p)) {
        score -= 0.4;
    }

    // Placeholder code in code tasks
    if matches!(intent.category, IntentCategory::CodeBuild | IntentCategory::CodeFix) {
        let placeholders = ["todo!()", "unimplemented!()", "// TODO", "pass  # TODO"];
        if placeholders.iter().any(|p| response.contains(p)) {
            score -= 0.3;
        }
    }

    // Suspiciously short for complex tasks
    if complexity == "complex" && response.len() < 200 {
        score -= 0.3;
    }

    // Refusal patterns
    if lower.contains("i can't") && lower.contains("instead") {
        score -= 0.3;
    }

    // Empty or near-empty
    if response.trim().len() < 20 {
        score -= 0.5;
    }

    score.max(0.0)
}

/// Suggest a prompt refinement based on detected issues.
fn suggest_prompt_refinement(
    response: &str,
    lower: &str,
    intent: &ClassifiedIntent,
) -> Option<String> {
    if lower.contains("i'm not sure") || lower.contains("i don't know") {
        return Some("Be more specific. If you need information, use available tools to find it.".into());
    }
    if response.contains("todo!()") || response.contains("unimplemented!()") {
        return Some("Provide complete implementations, not placeholders.".into());
    }
    if response.trim().len() < 100 && matches!(intent.category, IntentCategory::CodeBuild) {
        return Some("Provide a complete implementation with all necessary code.".into());
    }
    None
}

/// Suggest subtasks for decomposition.
fn suggest_decomposition(intent: &ClassifiedIntent) -> Vec<String> {
    match intent.category {
        IntentCategory::CodeBuild => vec![
            "Analyze requirements".into(),
            "Design interfaces/types".into(),
            "Implement core logic".into(),
            "Add error handling".into(),
            "Write tests".into(),
        ],
        IntentCategory::CodeFix => vec![
            "Reproduce the issue".into(),
            "Identify root cause".into(),
            "Design the fix".into(),
            "Apply and verify".into(),
        ],
        _ => vec![
            "Understand the request".into(),
            "Plan the approach".into(),
            "Execute step by step".into(),
            "Verify the result".into(),
        ],
    }
}

/// Get the next model in the escalation chain.
fn next_model(current: &str) -> Option<String> {
    if current.contains("haiku") { Some("claude-sonnet-4-6".into()) }
    else if current.contains("sonnet") { Some("claude-opus-4-6".into()) }
    else if current.contains("gpt-4o-mini") { Some("gpt-4o".into()) }
    else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_intent(cat: IntentCategory) -> ClassifiedIntent {
        ClassifiedIntent { category: cat, confidence: 0.9, target: None, payload: None }
    }

    #[test]
    fn test_no_escalation_good_response() {
        let p = escalate("Here is a complete implementation with all details.", &mock_intent(IntentCategory::CodeBuild), "simple", "claude-sonnet-4-6", 0);
        assert_eq!(p.level, EscalationLevel::None);
    }

    #[test]
    fn test_escalation_uncertain_response() {
        let p = escalate("I'm not sure how to do that.", &mock_intent(IntentCategory::CodeBuild), "complex", "claude-haiku-4-5-20251001", 0);
        assert!(p.level >= EscalationLevel::RetryPrompt);
    }

    #[test]
    fn test_escalation_third_attempt_human() {
        let p = escalate("Still failing.", &mock_intent(IntentCategory::CodeBuild), "complex", "claude-opus-4-6", 3);
        assert_eq!(p.level, EscalationLevel::HumanReview);
    }

    #[test]
    fn test_no_escalation_greeting() {
        let p = escalate("I don't know", &mock_intent(IntentCategory::Greeting), "simple", "claude-haiku-4-5-20251001", 0);
        assert_eq!(p.level, EscalationLevel::None);
    }

    #[test]
    fn test_decompose_at_max_model() {
        let p = escalate("todo!()", &mock_intent(IntentCategory::CodeBuild), "complex", "claude-opus-4-6", 2);
        assert_eq!(p.level, EscalationLevel::DecomposeTask);
        assert!(!p.subtasks.is_empty());
    }
}
