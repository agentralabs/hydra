//! Iterative planning — decomposes complex tasks into dependency-aware phases.
//!
//! UCU Module #2 (Wave 3). Replaces single-shot execution with phased plans.
//! Why not a sister? Uses Planning sister for persistence, but decomposition
//! itself is structural analysis (conjunctions, file refs) — no I/O needed.

use crate::cognitive::intent_router::{ClassifiedIntent, IntentCategory};

/// Execution phase of a task step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Understand the context and requirements.
    Analyze,
    /// Design the approach.
    Plan,
    /// Do the work.
    Execute,
    /// Check the result.
    Verify,
    /// Present to user.
    Deliver,
}

/// A single step in a task plan.
#[derive(Debug, Clone)]
pub struct TaskStep {
    pub id: usize,
    pub phase: Phase,
    pub description: String,
    /// Step IDs this depends on (must complete before this can start).
    pub depends_on: Vec<usize>,
    /// Estimated token cost for this step.
    pub estimated_tokens: u32,
    /// Sister to use for this step, if any.
    pub requires_sister: Option<String>,
}

/// A complete decomposed task plan.
#[derive(Debug, Clone)]
pub struct TaskPlan {
    pub steps: Vec<TaskStep>,
    pub complexity_assessment: String,
    pub estimated_total_tokens: u32,
    /// Groups of step IDs that can run in parallel.
    pub parallelizable_groups: Vec<Vec<usize>>,
}

/// Check whether a task warrants decomposition.
pub fn should_decompose(text: &str, complexity: &str) -> bool {
    if complexity == "simple" { return false; }

    // Multi-part requests (conjunctions)
    let multi_markers = [" and then ", " after that ", " also ", " additionally ",
        " first ", " second ", " third ", " finally ", " next "];
    let has_multi = multi_markers.iter().any(|m| text.to_lowercase().contains(m));

    // Numbered lists
    let has_numbered = text.contains("1.") && text.contains("2.");

    // Long input (likely complex)
    let is_long = text.len() > 500;

    has_multi || has_numbered || is_long || complexity == "complex"
}

/// Decompose a task into steps with dependencies.
pub fn decompose_task(
    text: &str,
    intent: &ClassifiedIntent,
    complexity: &str,
) -> TaskPlan {
    if !should_decompose(text, complexity) {
        return single_step_plan(text, intent);
    }

    match intent.category {
        IntentCategory::CodeBuild => decompose_code_build(text, complexity),
        IntentCategory::CodeFix => decompose_code_fix(text, complexity),
        IntentCategory::SelfImplement => decompose_self_implement(text),
        IntentCategory::Deploy => decompose_deploy(text),
        _ => decompose_generic(text, complexity),
    }
}

/// Simple single-step plan for tasks that don't need decomposition.
fn single_step_plan(text: &str, intent: &ClassifiedIntent) -> TaskPlan {
    let tokens = match intent.category {
        IntentCategory::Greeting | IntentCategory::Farewell => 500,
        IntentCategory::CodeBuild | IntentCategory::CodeFix => 4_000,
        _ => 2_000,
    };
    TaskPlan {
        steps: vec![TaskStep {
            id: 0, phase: Phase::Execute,
            description: text.chars().take(100).collect(),
            depends_on: vec![], estimated_tokens: tokens,
            requires_sister: None,
        }],
        complexity_assessment: "single_step".into(),
        estimated_total_tokens: tokens,
        parallelizable_groups: vec![vec![0]],
    }
}

fn decompose_code_build(text: &str, complexity: &str) -> TaskPlan {
    let tokens_per_step = if complexity == "complex" { 3_000 } else { 1_500 };
    let steps = vec![
        TaskStep { id: 0, phase: Phase::Analyze, description: "Analyze requirements and existing code".into(),
            depends_on: vec![], estimated_tokens: 1_000, requires_sister: Some("Codebase".into()) },
        TaskStep { id: 1, phase: Phase::Plan, description: "Design interfaces and architecture".into(),
            depends_on: vec![0], estimated_tokens: 2_000, requires_sister: Some("Forge".into()) },
        TaskStep { id: 2, phase: Phase::Execute, description: "Implement core logic".into(),
            depends_on: vec![1], estimated_tokens: tokens_per_step, requires_sister: None },
        TaskStep { id: 3, phase: Phase::Execute, description: "Implement error handling and edge cases".into(),
            depends_on: vec![2], estimated_tokens: tokens_per_step / 2, requires_sister: None },
        TaskStep { id: 4, phase: Phase::Verify, description: "Verify compilation and correctness".into(),
            depends_on: vec![3], estimated_tokens: 1_000, requires_sister: None },
    ];
    let total: u32 = steps.iter().map(|s| s.estimated_tokens).sum();
    TaskPlan {
        parallelizable_groups: vec![vec![0], vec![1], vec![2], vec![3], vec![4]],
        steps, complexity_assessment: format!("code_build_{}", complexity),
        estimated_total_tokens: total,
    }
}

fn decompose_code_fix(text: &str, _complexity: &str) -> TaskPlan {
    let steps = vec![
        TaskStep { id: 0, phase: Phase::Analyze, description: "Reproduce and understand the issue".into(),
            depends_on: vec![], estimated_tokens: 1_500, requires_sister: Some("Codebase".into()) },
        TaskStep { id: 1, phase: Phase::Plan, description: "Identify root cause".into(),
            depends_on: vec![0], estimated_tokens: 2_000, requires_sister: None },
        TaskStep { id: 2, phase: Phase::Execute, description: "Apply the fix".into(),
            depends_on: vec![1], estimated_tokens: 2_000, requires_sister: None },
        TaskStep { id: 3, phase: Phase::Verify, description: "Verify fix and check for regressions".into(),
            depends_on: vec![2], estimated_tokens: 1_000, requires_sister: None },
    ];
    let total: u32 = steps.iter().map(|s| s.estimated_tokens).sum();
    TaskPlan {
        parallelizable_groups: vec![vec![0], vec![1], vec![2], vec![3]],
        steps, complexity_assessment: "code_fix".into(),
        estimated_total_tokens: total,
    }
}

fn decompose_self_implement(text: &str) -> TaskPlan {
    let steps = vec![
        TaskStep { id: 0, phase: Phase::Analyze, description: "Analyze current system state".into(),
            depends_on: vec![], estimated_tokens: 2_000, requires_sister: Some("Codebase".into()) },
        TaskStep { id: 1, phase: Phase::Plan, description: "Design implementation plan".into(),
            depends_on: vec![0], estimated_tokens: 3_000, requires_sister: Some("Planning".into()) },
        TaskStep { id: 2, phase: Phase::Execute, description: "Implement changes".into(),
            depends_on: vec![1], estimated_tokens: 5_000, requires_sister: Some("Forge".into()) },
        TaskStep { id: 3, phase: Phase::Verify, description: "Compile and test".into(),
            depends_on: vec![2], estimated_tokens: 2_000, requires_sister: None },
        TaskStep { id: 4, phase: Phase::Deliver, description: "Report results".into(),
            depends_on: vec![3], estimated_tokens: 500, requires_sister: None },
    ];
    let total: u32 = steps.iter().map(|s| s.estimated_tokens).sum();
    TaskPlan {
        parallelizable_groups: vec![vec![0], vec![1], vec![2], vec![3], vec![4]],
        steps, complexity_assessment: "self_implement".into(),
        estimated_total_tokens: total,
    }
}

fn decompose_deploy(text: &str) -> TaskPlan {
    let steps = vec![
        TaskStep { id: 0, phase: Phase::Analyze, description: "Check deployment prerequisites".into(),
            depends_on: vec![], estimated_tokens: 1_000, requires_sister: Some("Reality".into()) },
        TaskStep { id: 1, phase: Phase::Plan, description: "Plan deployment and rollback strategy".into(),
            depends_on: vec![0], estimated_tokens: 1_500, requires_sister: Some("Aegis".into()) },
        TaskStep { id: 2, phase: Phase::Execute, description: "Execute deployment".into(),
            depends_on: vec![1], estimated_tokens: 2_000, requires_sister: None },
        TaskStep { id: 3, phase: Phase::Verify, description: "Verify deployment success".into(),
            depends_on: vec![2], estimated_tokens: 1_000, requires_sister: None },
    ];
    let total: u32 = steps.iter().map(|s| s.estimated_tokens).sum();
    TaskPlan {
        parallelizable_groups: vec![vec![0], vec![1], vec![2], vec![3]],
        steps, complexity_assessment: "deploy".into(),
        estimated_total_tokens: total,
    }
}

fn decompose_generic(text: &str, complexity: &str) -> TaskPlan {
    let tokens = if complexity == "complex" { 2_000 } else { 1_000 };
    let steps = vec![
        TaskStep { id: 0, phase: Phase::Analyze, description: "Understand the request".into(),
            depends_on: vec![], estimated_tokens: tokens, requires_sister: None },
        TaskStep { id: 1, phase: Phase::Execute, description: "Process and respond".into(),
            depends_on: vec![0], estimated_tokens: tokens * 2, requires_sister: None },
        TaskStep { id: 2, phase: Phase::Verify, description: "Verify completeness".into(),
            depends_on: vec![1], estimated_tokens: tokens / 2, requires_sister: None },
    ];
    let total: u32 = steps.iter().map(|s| s.estimated_tokens).sum();
    TaskPlan {
        parallelizable_groups: vec![vec![0], vec![1], vec![2]],
        steps, complexity_assessment: format!("generic_{}", complexity),
        estimated_total_tokens: total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_intent(cat: IntentCategory) -> ClassifiedIntent {
        ClassifiedIntent { category: cat, confidence: 0.9, target: None, payload: None }
    }

    #[test]
    fn test_simple_no_decompose() {
        assert!(!should_decompose("hello", "simple"));
    }

    #[test]
    fn test_complex_decomposes() {
        assert!(should_decompose("build a REST API and then deploy it", "complex"));
    }

    #[test]
    fn test_multi_part_decomposes() {
        assert!(should_decompose("first analyze the code and then fix the bug", "moderate"));
    }

    #[test]
    fn test_single_step_plan() {
        let plan = decompose_task("hello", &mock_intent(IntentCategory::Greeting), "simple");
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn test_code_build_plan() {
        let plan = decompose_task("build a REST API", &mock_intent(IntentCategory::CodeBuild), "complex");
        assert!(plan.steps.len() >= 4);
        // First step should have no dependencies
        assert!(plan.steps[0].depends_on.is_empty());
        // Later steps depend on earlier ones
        assert!(!plan.steps.last().unwrap().depends_on.is_empty());
    }

    #[test]
    fn test_plan_total_tokens() {
        let plan = decompose_task("fix this bug", &mock_intent(IntentCategory::CodeFix), "complex");
        let sum: u32 = plan.steps.iter().map(|s| s.estimated_tokens).sum();
        assert_eq!(sum, plan.estimated_total_tokens);
    }
}
