//! Agentic loop entry detection — determines if an interaction should use multi-turn.

use crate::cognitive::intent_router::{IntentCategory, ClassifiedIntent};
use crate::cognitive::runtime_settings::RuntimeSettings;
use super::agentic_loop_format::has_actionable_tags;

/// Configuration for the agentic loop, determined at entry.
#[derive(Debug, Clone)]
pub(crate) struct AgenticLoopConfig {
    pub max_turns: u8,
    pub turn_timeout_secs: u64,
    pub total_budget_tokens: u64,
}

/// Determine if this interaction should enter the agentic loop.
///
/// Returns None if single-pass is sufficient, or Some(config) if multi-turn is warranted.
/// Used by phase_act for full intent-aware entry; simpler inline check also exists.
#[allow(dead_code)]
pub(crate) fn should_enter_agentic_loop(
    intent: &ClassifiedIntent,
    complexity: &str,
    initial_response: &str,
    runtime: &RuntimeSettings,
) -> Option<AgenticLoopConfig> {
    // Global kill switch
    if !runtime.agentic_loop { return None; }

    // Must have actionable tags in the initial response
    if !has_actionable_tags(initial_response) { return None; }

    // Configure based on intent category
    let config = match intent.category {
        // Code tasks: high iteration potential
        IntentCategory::CodeBuild | IntentCategory::CodeFix => AgenticLoopConfig {
            max_turns: runtime.agentic_max_turns.min(10),
            turn_timeout_secs: 30,
            total_budget_tokens: runtime.agentic_token_budget,
        },

        // Deploy: needs verification steps
        IntentCategory::Deploy => AgenticLoopConfig {
            max_turns: runtime.agentic_max_turns.min(10),
            turn_timeout_secs: 45,
            total_budget_tokens: runtime.agentic_token_budget,
        },

        // Self-implement: complex multi-step
        IntentCategory::SelfImplement => AgenticLoopConfig {
            max_turns: runtime.agentic_max_turns.min(12),
            turn_timeout_secs: 60,
            total_budget_tokens: runtime.agentic_token_budget,
        },

        // Web browsing: limited iteration
        IntentCategory::WebBrowse => AgenticLoopConfig {
            max_turns: runtime.agentic_max_turns.min(5),
            turn_timeout_secs: 30,
            total_budget_tokens: runtime.agentic_token_budget / 2,
        },

        // Code explain, file ops, planning: moderate
        IntentCategory::CodeExplain | IntentCategory::FileOperation
        | IntentCategory::PlanningQuery => {
            if complexity != "complex" { return None; }
            AgenticLoopConfig {
                max_turns: runtime.agentic_max_turns.min(4),
                turn_timeout_secs: 30,
                total_budget_tokens: runtime.agentic_token_budget / 2,
            }
        }

        // Everything else: only if complex AND has tools
        _ => {
            if complexity != "complex" { return None; }
            AgenticLoopConfig {
                max_turns: runtime.agentic_max_turns.min(3),
                turn_timeout_secs: 30,
                total_budget_tokens: runtime.agentic_token_budget / 3,
            }
        }
    };

    Some(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_intent(cat: IntentCategory) -> ClassifiedIntent {
        ClassifiedIntent { category: cat, confidence: 0.9, target: None, payload: None }
    }

    #[test]
    fn test_no_loop_without_tags() {
        let rt = RuntimeSettings::default();
        let result = should_enter_agentic_loop(
            &mock_intent(IntentCategory::CodeBuild), "complex", "plain text", &rt,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_loop_for_code_with_tags() {
        let rt = RuntimeSettings::default();
        let result = should_enter_agentic_loop(
            &mock_intent(IntentCategory::CodeBuild), "complex",
            r#"<hydra-tool name="x">{}</hydra-tool>"#, &rt,
        );
        assert!(result.is_some());
        assert!(result.unwrap().max_turns <= 10);
    }

    #[test]
    fn test_no_loop_when_disabled() {
        let rt = RuntimeSettings { agentic_loop: false, ..Default::default() };
        let result = should_enter_agentic_loop(
            &mock_intent(IntentCategory::CodeBuild), "complex",
            "<hydra-exec>ls</hydra-exec>", &rt,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_simple_greeting_no_loop() {
        let rt = RuntimeSettings::default();
        let result = should_enter_agentic_loop(
            &mock_intent(IntentCategory::Greeting), "simple",
            "<hydra-exec>echo hi</hydra-exec>", &rt,
        );
        assert!(result.is_none());
    }
}
