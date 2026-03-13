//! Self-improvement engine — identify weaknesses and generate improvement specs.
//!
//! Phase 6 of the superintelligence plan. Uses outcome data to identify
//! categories where Hydra performs poorly, then generates improvement
//! candidates (prompt enhancements, belief injections, model upgrades).

/// A weakness identified from outcome tracking data.
#[derive(Debug, Clone)]
pub struct ImprovementCandidate {
    pub weakness: String,
    pub category: String,        // IntentCategory debug name
    pub success_rate: f64,
    pub sample_count: u64,
    pub suggested_fix: ImprovementType,
}

/// Types of self-improvements Hydra can apply.
#[derive(Debug, Clone)]
pub enum ImprovementType {
    /// Add patterns or context to the system prompt for this category
    PromptEnhancement(String),
    /// Inject corrective beliefs into the belief store
    BeliefInjection { subject: String, content: String, confidence: f64 },
    /// Route different tools for this category
    ToolRouteChange { add_tools: Vec<String>, remove_tools: Vec<String> },
    /// Default to a stronger model for this category
    ModelUpgrade(String),
}

/// Identify weaknesses from category success rates.
///
/// Takes a list of (category_name, success_rate, sample_count) tuples
/// from the OutcomeTracker and returns improvement candidates.
pub fn identify_weaknesses(
    category_stats: &[(String, f64, u64)],
    min_interactions: u64,
    max_success_rate: f64,
) -> Vec<ImprovementCandidate> {
    let mut candidates = Vec::new();

    for (category, rate, count) in category_stats {
        if *count < min_interactions { continue; }
        if *rate >= max_success_rate { continue; }

        let fix = suggest_fix(category, *rate);
        candidates.push(ImprovementCandidate {
            weakness: format!("{} has {:.0}% success rate ({} interactions)", category, rate * 100.0, count),
            category: category.clone(),
            success_rate: *rate,
            sample_count: *count,
            suggested_fix: fix,
        });
    }

    // Sort by worst success rate first
    candidates.sort_by(|a, b| a.success_rate.partial_cmp(&b.success_rate).unwrap_or(std::cmp::Ordering::Equal));
    candidates
}

/// Suggest the most appropriate fix for a given category weakness.
fn suggest_fix(category: &str, success_rate: f64) -> ImprovementType {
    // Very low success → model upgrade (the model can't handle it)
    if success_rate < 0.3 {
        return ImprovementType::ModelUpgrade("claude-sonnet-4-6".into());
    }

    // Code categories → prompt enhancement with patterns
    if category.contains("Code") {
        return ImprovementType::PromptEnhancement(format!(
            "When handling {} tasks, be extra careful to: \
             1) Read existing code before modifying, \
             2) Check for edge cases, \
             3) Verify your solution compiles/runs.",
            category
        ));
    }

    // Deploy → belief injection about safety
    if category.contains("Deploy") {
        return ImprovementType::BeliefInjection {
            subject: "deployment_safety".into(),
            content: "Always verify deployment commands are reversible and have rollback plans.".into(),
            confidence: 0.9,
        };
    }

    // Default: prompt enhancement
    ImprovementType::PromptEnhancement(format!(
        "Historical data shows low success rate for {} tasks. \
         Take extra care and verify your response before delivering.",
        category
    ))
}

/// Generate a markdown spec from an improvement candidate.
/// This spec can be fed into the SelfImplement pipeline.
pub fn generate_improvement_spec(candidate: &ImprovementCandidate) -> String {
    let mut spec = String::new();
    spec.push_str(&format!("# Self-Improvement Spec: {}\n\n", candidate.category));
    spec.push_str(&format!("## Problem\n\n{}\n\n", candidate.weakness));
    spec.push_str("## Proposed Fix\n\n");

    match &candidate.suggested_fix {
        ImprovementType::PromptEnhancement(prompt) => {
            spec.push_str("**Type**: Prompt Enhancement\n\n");
            spec.push_str(&format!("Add to system prompt for {} tasks:\n\n", candidate.category));
            spec.push_str(&format!("> {}\n\n", prompt));
        }
        ImprovementType::BeliefInjection { subject, content, confidence } => {
            spec.push_str("**Type**: Belief Injection\n\n");
            spec.push_str(&format!("Add belief:\n- Subject: `{}`\n- Content: {}\n- Confidence: {:.0}%\n\n",
                subject, content, confidence * 100.0));
        }
        ImprovementType::ToolRouteChange { add_tools, remove_tools } => {
            spec.push_str("**Type**: Tool Route Change\n\n");
            if !add_tools.is_empty() {
                spec.push_str(&format!("Add tools: {}\n", add_tools.join(", ")));
            }
            if !remove_tools.is_empty() {
                spec.push_str(&format!("Remove tools: {}\n", remove_tools.join(", ")));
            }
            spec.push('\n');
        }
        ImprovementType::ModelUpgrade(model) => {
            spec.push_str("**Type**: Model Upgrade\n\n");
            spec.push_str(&format!("Default to `{}` for {} tasks.\n\n", model, candidate.category));
        }
    }

    spec.push_str("## Verification\n\n");
    spec.push_str("After applying this improvement:\n");
    spec.push_str("1. Track success rate for the next 20 interactions in this category\n");
    spec.push_str("2. If success rate improves by >= 10%, keep the change\n");
    spec.push_str("3. If success rate drops, revert immediately\n");

    spec
}

/// Check if an improvement has been successful by comparing before/after rates.
pub fn evaluate_improvement(
    before_rate: f64,
    after_rate: f64,
    min_improvement: f64,
) -> ImprovementOutcome {
    let delta = after_rate - before_rate;
    if delta >= min_improvement {
        ImprovementOutcome::Success { delta }
    } else if delta < -0.05 {
        ImprovementOutcome::Regression { delta }
    } else {
        ImprovementOutcome::Inconclusive { delta }
    }
}

/// Outcome of evaluating a self-improvement.
#[derive(Debug, Clone)]
pub enum ImprovementOutcome {
    Success { delta: f64 },
    Regression { delta: f64 },
    Inconclusive { delta: f64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identify_weaknesses() {
        let stats = vec![
            ("CodeBuild".into(), 0.85, 50u64),  // Good — won't be flagged
            ("Deploy".into(), 0.35, 20),          // Bad — will be flagged
            ("Greeting".into(), 0.95, 100),       // Great — won't be flagged
            ("WebBrowse".into(), 0.45, 15),       // Bad — will be flagged
            ("CodeFix".into(), 0.20, 5),           // Bad but too few interactions
        ];
        let candidates = identify_weaknesses(&stats, 10, 0.6);
        assert_eq!(candidates.len(), 2);
        assert!(candidates[0].success_rate < candidates[1].success_rate); // Sorted worst first
    }

    #[test]
    fn test_suggest_fix_low_rate() {
        let fix = suggest_fix("CodeBuild", 0.2);
        assert!(matches!(fix, ImprovementType::ModelUpgrade(_)));
    }

    #[test]
    fn test_suggest_fix_code_category() {
        let fix = suggest_fix("CodeBuild", 0.5);
        assert!(matches!(fix, ImprovementType::PromptEnhancement(_)));
    }

    #[test]
    fn test_suggest_fix_deploy() {
        let fix = suggest_fix("Deploy", 0.5);
        assert!(matches!(fix, ImprovementType::BeliefInjection { .. }));
    }

    #[test]
    fn test_generate_spec_prompt_enhancement() {
        let candidate = ImprovementCandidate {
            weakness: "CodeBuild has 50% success rate".into(),
            category: "CodeBuild".into(),
            success_rate: 0.5,
            sample_count: 30,
            suggested_fix: ImprovementType::PromptEnhancement("Be more careful".into()),
        };
        let spec = generate_improvement_spec(&candidate);
        assert!(spec.contains("# Self-Improvement Spec"));
        assert!(spec.contains("Prompt Enhancement"));
        assert!(spec.contains("Verification"));
    }

    #[test]
    fn test_generate_spec_model_upgrade() {
        let candidate = ImprovementCandidate {
            weakness: "Deploy has 20% success rate".into(),
            category: "Deploy".into(),
            success_rate: 0.2,
            sample_count: 15,
            suggested_fix: ImprovementType::ModelUpgrade("claude-sonnet-4-6".into()),
        };
        let spec = generate_improvement_spec(&candidate);
        assert!(spec.contains("Model Upgrade"));
        assert!(spec.contains("claude-sonnet-4-6"));
    }

    #[test]
    fn test_evaluate_improvement_success() {
        let outcome = evaluate_improvement(0.4, 0.6, 0.1);
        assert!(matches!(outcome, ImprovementOutcome::Success { .. }));
    }

    #[test]
    fn test_evaluate_improvement_regression() {
        let outcome = evaluate_improvement(0.6, 0.4, 0.1);
        assert!(matches!(outcome, ImprovementOutcome::Regression { .. }));
    }

    #[test]
    fn test_evaluate_improvement_inconclusive() {
        let outcome = evaluate_improvement(0.5, 0.52, 0.1);
        assert!(matches!(outcome, ImprovementOutcome::Inconclusive { .. }));
    }

    #[test]
    fn test_empty_stats() {
        let candidates = identify_weaknesses(&[], 10, 0.6);
        assert!(candidates.is_empty());
    }
}
