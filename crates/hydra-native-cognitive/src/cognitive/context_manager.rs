//! Tier-aware context management — right-sizes prompt content for each model tier.
//!
//! UCU Module #7 (Wave 1 — foundation). Every other UCU module consumes ModelTier.
//! Why not a sister? Purely in-memory prompt assembly — no I/O, no classification.

use crate::cognitive::intent_router::{ClassifiedIntent, IntentCategory};

/// Model capability tier — determines context window and prompt strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    /// ~200K context, fast, cheap. Use for simple tasks.
    Haiku,
    /// ~200K context, balanced. Use for moderate tasks.
    Sonnet,
    /// ~200K context, strongest reasoning. Use for complex tasks.
    Opus,
}

/// Budget constraints for context assembly.
#[derive(Debug, Clone)]
pub struct ContextBudget {
    /// Max characters for the system prompt (before truncation).
    pub system_prompt_max_chars: usize,
    /// Max conversation history turns to include.
    pub history_limit: usize,
    /// Max characters per history message.
    pub max_msg_chars: usize,
    /// Whether to include belief context.
    pub include_beliefs: bool,
    /// Whether to include federation status.
    pub include_federation: bool,
    /// Whether to include code index context.
    pub include_code_index: bool,
    /// Whether to include skills context.
    pub include_skills: bool,
    /// Whether to include full tool routing.
    pub include_tools: bool,
}

/// Result of context assembly — ready for LLM call.
#[derive(Debug, Clone)]
pub struct AssembledContext {
    /// Trimmed conversation history.
    pub history: Vec<(String, String)>,
    /// Estimated total tokens (system + history + user message).
    pub estimated_tokens: usize,
    /// Which tier was used.
    pub tier: ModelTier,
    /// The budget that was applied.
    pub budget: ContextBudget,
}

/// Parse a model name string into its capability tier.
pub fn tier_from_model(model_name: &str) -> ModelTier {
    let lower = model_name.to_lowercase();
    if lower.contains("haiku") || lower.contains("gpt-4o-mini") || lower.contains("gemini-flash") {
        ModelTier::Haiku
    } else if lower.contains("opus") || lower.contains("o1") || lower.contains("o3") {
        ModelTier::Opus
    } else {
        // Sonnet, GPT-4o, Gemini Pro, and anything else → middle tier
        ModelTier::Sonnet
    }
}

/// Build a context budget appropriate for the model tier and task.
pub fn budget_for_tier(
    tier: ModelTier,
    intent: &ClassifiedIntent,
    complexity: &str,
) -> ContextBudget {
    let is_code = matches!(intent.category,
        IntentCategory::CodeBuild | IntentCategory::CodeFix | IntentCategory::CodeExplain
    );

    match tier {
        ModelTier::Haiku => ContextBudget {
            system_prompt_max_chars: 8_000,
            history_limit: if is_code { 4 } else { 6 },
            max_msg_chars: 500,
            include_beliefs: complexity == "complex",
            include_federation: false,
            include_code_index: is_code,
            include_skills: false,
            include_tools: is_code,
        },
        ModelTier::Sonnet => ContextBudget {
            system_prompt_max_chars: 30_000,
            history_limit: if is_code { 10 } else { 15 },
            max_msg_chars: 2_000,
            include_beliefs: true,
            include_federation: false,
            include_code_index: is_code,
            include_skills: true,
            include_tools: true,
        },
        ModelTier::Opus => ContextBudget {
            system_prompt_max_chars: 120_000,
            history_limit: 25,
            max_msg_chars: 8_000,
            include_beliefs: true,
            include_federation: true,
            include_code_index: true,
            include_skills: true,
            include_tools: true,
        },
    }
}

/// Assemble context by trimming history according to the budget.
pub fn assemble_context(
    tier: ModelTier,
    budget: &ContextBudget,
    history: &[(String, String)],
    system_prompt_len: usize,
) -> AssembledContext {
    // Trim history to budget limits
    let trimmed: Vec<(String, String)> = history
        .iter()
        .rev()
        .take(budget.history_limit)
        .rev()
        .map(|(role, content)| {
            let truncated = if content.len() > budget.max_msg_chars {
                format!("{}...", &content[..budget.max_msg_chars])
            } else {
                content.clone()
            };
            (role.clone(), truncated)
        })
        .collect();

    // Estimate tokens
    let prompt_tokens = token_estimate_chars(system_prompt_len);
    let history_tokens: usize = trimmed.iter()
        .map(|(_, c)| token_estimate_chars(c.len()))
        .sum();
    let estimated = prompt_tokens + history_tokens;

    AssembledContext {
        history: trimmed,
        estimated_tokens: estimated,
        tier,
        budget: budget.clone(),
    }
}

/// Estimate token count from character count.
/// Rough heuristic: ~4 chars per token for English text.
pub fn token_estimate_chars(char_count: usize) -> usize {
    (char_count + 3) / 4
}

/// Get the maximum context window size for a tier (in tokens).
pub fn max_context_tokens(tier: ModelTier) -> usize {
    match tier {
        ModelTier::Haiku => 200_000,
        ModelTier::Sonnet => 200_000,
        ModelTier::Opus => 200_000,
    }
}

/// Get a human-readable tier name.
pub fn tier_name(tier: ModelTier) -> &'static str {
    match tier {
        ModelTier::Haiku => "haiku",
        ModelTier::Sonnet => "sonnet",
        ModelTier::Opus => "opus",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_intent(cat: IntentCategory) -> ClassifiedIntent {
        ClassifiedIntent { category: cat, confidence: 0.9, target: None, payload: None }
    }

    #[test]
    fn test_tier_from_model() {
        assert_eq!(tier_from_model("claude-haiku-4-5-20251001"), ModelTier::Haiku);
        assert_eq!(tier_from_model("claude-sonnet-4-6"), ModelTier::Sonnet);
        assert_eq!(tier_from_model("claude-opus-4-6"), ModelTier::Opus);
        assert_eq!(tier_from_model("gpt-4o-mini"), ModelTier::Haiku);
        assert_eq!(tier_from_model("gpt-4o"), ModelTier::Sonnet);
        assert_eq!(tier_from_model("unknown-model"), ModelTier::Sonnet);
    }

    #[test]
    fn test_budget_haiku_compact() {
        let intent = mock_intent(IntentCategory::Greeting);
        let budget = budget_for_tier(ModelTier::Haiku, &intent, "simple");
        assert_eq!(budget.system_prompt_max_chars, 8_000);
        assert_eq!(budget.history_limit, 6);
        assert!(!budget.include_beliefs);
        assert!(!budget.include_federation);
    }

    #[test]
    fn test_budget_opus_full() {
        let intent = mock_intent(IntentCategory::CodeBuild);
        let budget = budget_for_tier(ModelTier::Opus, &intent, "complex");
        assert_eq!(budget.system_prompt_max_chars, 120_000);
        assert_eq!(budget.history_limit, 25);
        assert!(budget.include_beliefs);
        assert!(budget.include_federation);
        assert!(budget.include_code_index);
    }

    #[test]
    fn test_assemble_trims_history() {
        let budget = ContextBudget {
            system_prompt_max_chars: 8000,
            history_limit: 2,
            max_msg_chars: 100,
            include_beliefs: false,
            include_federation: false,
            include_code_index: false,
            include_skills: false,
            include_tools: false,
        };
        let history = vec![
            ("user".into(), "msg1".into()),
            ("hydra".into(), "msg2".into()),
            ("user".into(), "msg3".into()),
            ("hydra".into(), "msg4".into()),
        ];
        let ctx = assemble_context(ModelTier::Haiku, &budget, &history, 1000);
        assert_eq!(ctx.history.len(), 2); // Only last 2
        assert_eq!(ctx.history[0].1, "msg3");
        assert_eq!(ctx.history[1].1, "msg4");
    }

    #[test]
    fn test_assemble_truncates_long_messages() {
        let budget = ContextBudget {
            system_prompt_max_chars: 8000,
            history_limit: 10,
            max_msg_chars: 10,
            include_beliefs: false,
            include_federation: false,
            include_code_index: false,
            include_skills: false,
            include_tools: false,
        };
        let history = vec![("user".into(), "a".repeat(100))];
        let ctx = assemble_context(ModelTier::Haiku, &budget, &history, 500);
        assert!(ctx.history[0].1.len() <= 14); // 10 + "..."
    }

    #[test]
    fn test_token_estimate() {
        assert_eq!(token_estimate_chars(100), 25);
        assert_eq!(token_estimate_chars(0), 0);
        assert_eq!(token_estimate_chars(1), 1);
    }
}
