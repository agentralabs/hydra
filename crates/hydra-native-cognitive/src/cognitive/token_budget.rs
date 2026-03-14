//! Dynamic token budgeting — right-sizes LLM calls based on task complexity.
//!
//! UCU Module #1 (Wave 2). Replaces fixed token allocation with adaptive budgets.
//! Why not a sister? Purely in-memory computation — no I/O, no classification.

use crate::cognitive::context_manager::ModelTier;
use crate::cognitive::intent_router::IntentCategory;

/// Contextual information about the current task for budget estimation.
#[derive(Debug, Clone)]
pub struct TaskContext {
    pub intent: IntentCategory,
    pub complexity: String,
    pub is_action: bool,
    pub history_length: usize,
    pub model_tier: ModelTier,
    pub has_memory_context: bool,
    pub iteration: u8,
    pub runtime_budget: u64,
}

/// Computed token budget for a single LLM call.
#[derive(Debug, Clone)]
pub struct TokenBudget {
    /// Max output tokens to request from the LLM.
    pub max_output_tokens: u32,
    /// Suggested temperature for this task type.
    pub temperature: f32,
    /// Brief explanation of why this budget was chosen.
    pub reasoning: &'static str,
}

/// Estimate the optimal token budget for a task.
pub fn estimate_budget(ctx: &TaskContext) -> TokenBudget {
    let base = base_budget(&ctx.intent, &ctx.complexity);

    // Scale by model tier — bigger models can produce more useful output
    let tier_multiplier = match ctx.model_tier {
        ModelTier::Haiku => 0.6,
        ModelTier::Sonnet => 1.0,
        ModelTier::Opus => 1.5,
    };

    // Scale up on retries — failed attempts need more context/output
    let retry_multiplier = 1.0 + (ctx.iteration as f64 * 0.3);

    // Scale for action requests (need room for command output analysis)
    let action_multiplier = if ctx.is_action { 1.3 } else { 1.0 };

    let scaled = (base.0 as f64 * tier_multiplier * retry_multiplier * action_multiplier) as u32;

    // Clamp to runtime budget
    let clamped = scaled.min(ctx.runtime_budget as u32).max(256);

    TokenBudget {
        max_output_tokens: clamped,
        temperature: base.1,
        reasoning: base.2,
    }
}

/// Base budget by intent + complexity. Returns (tokens, temperature, reason).
fn base_budget(intent: &IntentCategory, complexity: &str) -> (u32, f32, &'static str) {
    match intent {
        // Greetings/social — minimal
        IntentCategory::Greeting | IntentCategory::Farewell | IntentCategory::Thanks =>
            (512, 0.7, "social_interaction"),

        // Code tasks — scale heavily with complexity
        IntentCategory::CodeBuild => match complexity {
            "complex" => (8_000, 0.3, "complex_code_generation"),
            "moderate" => (4_000, 0.3, "moderate_code_generation"),
            _ => (2_000, 0.4, "simple_code_generation"),
        },
        IntentCategory::CodeFix => match complexity {
            "complex" => (6_000, 0.2, "complex_bug_fix"),
            "moderate" => (3_000, 0.3, "moderate_bug_fix"),
            _ => (1_500, 0.3, "simple_bug_fix"),
        },
        IntentCategory::CodeExplain => match complexity {
            "complex" => (4_000, 0.5, "complex_explanation"),
            _ => (2_000, 0.5, "code_explanation"),
        },

        // Deploy — medium, safety-focused
        IntentCategory::Deploy => (3_000, 0.2, "deployment_command"),

        // Self-implement — large budget for architectural changes
        IntentCategory::SelfImplement => (8_000, 0.3, "self_implementation"),

        // Memory operations — small
        IntentCategory::MemoryStore | IntentCategory::MemoryRecall =>
            (1_000, 0.4, "memory_operation"),

        // Settings — tiny
        IntentCategory::Settings => (500, 0.3, "settings_change"),

        // Conversation — moderate
        IntentCategory::Question => match complexity {
            "complex" => (4_000, 0.6, "complex_conversation"),
            "moderate" => (2_000, 0.6, "moderate_conversation"),
            _ => (1_000, 0.7, "simple_conversation"),
        },

        // Default — moderate
        _ => match complexity {
            "complex" => (4_000, 0.5, "complex_task"),
            "moderate" => (2_000, 0.5, "moderate_task"),
            _ => (1_000, 0.5, "simple_task"),
        },
    }
}

/// Estimate budget for an agentic loop iteration.
pub fn agentic_iteration_budget(
    iteration: u8,
    max_iterations: u8,
    total_budget: u64,
    tokens_used: u64,
) -> u32 {
    let remaining = total_budget.saturating_sub(tokens_used);
    let remaining_iterations = (max_iterations - iteration).max(1) as u64;
    // Distribute remaining budget evenly, with a small bonus for early iterations
    let per_iter = remaining / remaining_iterations;
    let early_bonus = if iteration < 3 { per_iter / 4 } else { 0 };
    (per_iter + early_bonus).min(16_000) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greeting_small_budget() {
        let ctx = TaskContext {
            intent: IntentCategory::Greeting,
            complexity: "simple".into(),
            is_action: false,
            history_length: 0,
            model_tier: ModelTier::Sonnet,
            has_memory_context: false,
            iteration: 0,
            runtime_budget: 50_000,
        };
        let budget = estimate_budget(&ctx);
        assert!(budget.max_output_tokens <= 600);
    }

    #[test]
    fn test_complex_code_large_budget() {
        let ctx = TaskContext {
            intent: IntentCategory::CodeBuild,
            complexity: "complex".into(),
            is_action: false,
            history_length: 5,
            model_tier: ModelTier::Opus,
            has_memory_context: true,
            iteration: 0,
            runtime_budget: 50_000,
        };
        let budget = estimate_budget(&ctx);
        assert!(budget.max_output_tokens >= 8_000);
    }

    #[test]
    fn test_retry_increases_budget() {
        let base = TaskContext {
            intent: IntentCategory::CodeFix,
            complexity: "moderate".into(),
            is_action: false,
            history_length: 3,
            model_tier: ModelTier::Sonnet,
            has_memory_context: false,
            iteration: 0,
            runtime_budget: 50_000,
        };
        let b0 = estimate_budget(&base);
        let mut retry = base.clone();
        retry.iteration = 2;
        let b2 = estimate_budget(&retry);
        assert!(b2.max_output_tokens > b0.max_output_tokens);
    }

    #[test]
    fn test_haiku_gets_smaller_budget() {
        let sonnet = TaskContext {
            intent: IntentCategory::Question,
            complexity: "moderate".into(),
            is_action: false, history_length: 3,
            model_tier: ModelTier::Sonnet,
            has_memory_context: false, iteration: 0,
            runtime_budget: 50_000,
        };
        let mut haiku = sonnet.clone();
        haiku.model_tier = ModelTier::Haiku;
        assert!(estimate_budget(&haiku).max_output_tokens < estimate_budget(&sonnet).max_output_tokens);
    }

    #[test]
    fn test_agentic_iteration_budget() {
        let b = agentic_iteration_budget(0, 8, 50_000, 0);
        assert!(b > 5_000);
        let b_late = agentic_iteration_budget(6, 8, 50_000, 40_000);
        assert!(b_late <= 10_000);
    }
}
