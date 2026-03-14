//! Outcome validation — verifies actual results match expected outcomes.
//!
//! UCU Module #13 (Wave 4). Complements verify_response.rs (factual claims)
//! with functional outcome checking: did the action achieve the user's goal?
//! Why not a sister? Core validation is string/struct analysis. Sisters used
//! for deep verification only when available (via existing verify_response).

use crate::cognitive::intent_router::IntentCategory;
use crate::cognitive::iterative_planner::TaskPlan;

/// Result of outcome validation.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub score: f32,
    pub checks_passed: Vec<String>,
    pub checks_failed: Vec<String>,
    /// Suggested outcome for OutcomeTracker.
    pub suggested_outcome: SuggestedOutcome,
}

/// Outcome suggestion for the learning phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestedOutcome {
    Success,
    PartialSuccess,
    Failure,
}

/// Context for validation.
pub struct ValidationContext<'a> {
    pub user_input: &'a str,
    pub intent: IntentCategory,
    pub response: &'a str,
    /// (command, output, success) tuples from hydra-exec.
    pub exec_results: &'a [(String, String, bool)],
    pub plan: Option<&'a TaskPlan>,
    pub llm_ok: bool,
}

/// Run all applicable validation checks on the cognitive loop output.
pub fn validate_outcome(ctx: &ValidationContext<'_>) -> ValidationResult {
    let mut passed = Vec::new();
    let mut failed = Vec::new();

    // 1. LLM call succeeded
    if ctx.llm_ok {
        passed.push("LLM call completed successfully".into());
    } else {
        failed.push("LLM call failed".into());
    }

    // 2. Response is non-empty and substantive
    let (resp_ok, resp_msg) = check_response_substance(ctx.response);
    if resp_ok { passed.push(resp_msg); } else { failed.push(resp_msg); }

    // 3. Response addresses the request
    let (addr_ok, addr_msg) = check_addresses_request(ctx.response, ctx.user_input, ctx.intent);
    if addr_ok { passed.push(addr_msg); } else { failed.push(addr_msg); }

    // 4. Command executions succeeded (if any)
    if !ctx.exec_results.is_empty() {
        let (exec_ok, exec_msg) = check_exec_success(ctx.exec_results);
        if exec_ok { passed.push(exec_msg); } else { failed.push(exec_msg); }
    }

    // 5. Plan coverage (if decomposed)
    if let Some(plan) = ctx.plan {
        let completed: Vec<usize> = (0..plan.steps.len()).collect(); // Assume all completed if we got here
        let (plan_ok, plan_msg) = check_plan_coverage(plan, &completed);
        if plan_ok { passed.push(plan_msg); } else { failed.push(plan_msg); }
    }

    // 6. Code tasks: check for code in response
    if matches!(ctx.intent, IntentCategory::CodeBuild | IntentCategory::CodeFix) {
        let (code_ok, code_msg) = check_code_present(ctx.response, ctx.intent);
        if code_ok { passed.push(code_msg); } else { failed.push(code_msg); }
    }

    // Score and summarize
    let total = (passed.len() + failed.len()) as f32;
    let score = if total > 0.0 { passed.len() as f32 / total } else { 0.5 };

    let suggested = if failed.is_empty() {
        SuggestedOutcome::Success
    } else if passed.len() > failed.len() {
        SuggestedOutcome::PartialSuccess
    } else {
        SuggestedOutcome::Failure
    };

    ValidationResult {
        valid: score >= 0.5 && !failed.iter().any(|f| f.contains("LLM call failed")),
        score,
        checks_passed: passed,
        checks_failed: failed,
        suggested_outcome: suggested,
    }
}

/// Check that the response has actual substance.
fn check_response_substance(response: &str) -> (bool, String) {
    let trimmed = response.trim();
    if trimmed.is_empty() {
        return (false, "Response is empty".into());
    }
    if trimmed.len() < 20 {
        return (false, format!("Response too short ({} chars)", trimmed.len()));
    }

    // Check for pure apology/deflection
    let lower = trimmed.to_lowercase();
    let deflections = ["i apologize", "i'm sorry but i can't", "i cannot help with"];
    if deflections.iter().any(|d| lower.starts_with(d)) && trimmed.len() < 200 {
        return (false, "Response is a deflection without substance".into());
    }

    (true, "Response has substance".into())
}

/// Check that the response addresses the user's request.
fn check_addresses_request(response: &str, input: &str, intent: IntentCategory) -> (bool, String) {
    let response_lower = response.to_lowercase();
    let input_lower = input.to_lowercase();

    // Extract key words from input (4+ chars, not stop words)
    let stop_words = ["that", "this", "what", "with", "from", "have", "been",
        "they", "will", "would", "could", "should", "about", "there", "their"];
    let input_keywords: Vec<&str> = input_lower.split_whitespace()
        .filter(|w| w.len() >= 4 && !stop_words.contains(w))
        .collect();

    if input_keywords.is_empty() {
        return (true, "Request addressed (no key terms to match)".into());
    }

    let overlap = input_keywords.iter()
        .filter(|w| response_lower.contains(*w))
        .count();
    let ratio = overlap as f32 / input_keywords.len() as f32;

    if ratio >= 0.2 {
        (true, format!("Response addresses request ({:.0}% keyword overlap)", ratio * 100.0))
    } else {
        (false, format!("Response may not address request ({:.0}% keyword overlap)", ratio * 100.0))
    }
}

/// Check whether command executions succeeded.
fn check_exec_success(exec_results: &[(String, String, bool)]) -> (bool, String) {
    let total = exec_results.len();
    let succeeded = exec_results.iter().filter(|(_, _, ok)| *ok).count();

    if succeeded == total {
        (true, format!("All {} command(s) succeeded", total))
    } else if succeeded > 0 {
        (false, format!("{}/{} commands succeeded", succeeded, total))
    } else {
        (false, format!("All {} command(s) failed", total))
    }
}

/// Check plan step coverage.
fn check_plan_coverage(plan: &TaskPlan, completed_ids: &[usize]) -> (bool, String) {
    let total = plan.steps.len();
    let covered = completed_ids.len().min(total);

    if covered == total {
        (true, format!("All {} plan steps completed", total))
    } else {
        (false, format!("{}/{} plan steps completed", covered, total))
    }
}

/// Check that code tasks include actual code in the response.
fn check_code_present(response: &str, intent: IntentCategory) -> (bool, String) {
    // Look for code block markers
    let has_code_block = response.contains("```") || response.contains("<hydra-exec>");

    // Look for code-like patterns
    let code_indicators = ["fn ", "def ", "class ", "function ", "const ", "let ", "var ",
        "import ", "from ", "pub ", "struct ", "impl ", "async ", "return "];
    let has_code_pattern = code_indicators.iter().any(|p| response.contains(p));

    if has_code_block || has_code_pattern {
        (true, "Response includes code".into())
    } else if matches!(intent, IntentCategory::CodeExplain) {
        // Explanations don't always need code
        (true, "Code explanation (code block optional)".into())
    } else {
        (false, "Code task response lacks code".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_good_outcome() {
        let result = validate_outcome(&ValidationContext {
            user_input: "How does Rust handle memory?",
            intent: IntentCategory::Question,
            response: "Rust uses an ownership system with borrowing and lifetimes to manage memory safely without a garbage collector.",
            exec_results: &[],
            plan: None,
            llm_ok: true,
        });
        assert!(result.valid);
        assert!(result.score >= 0.7);
        assert_eq!(result.suggested_outcome, SuggestedOutcome::Success);
    }

    #[test]
    fn test_empty_response() {
        let result = validate_outcome(&ValidationContext {
            user_input: "test", intent: IntentCategory::Question,
            response: "", exec_results: &[], plan: None, llm_ok: true,
        });
        assert!(!result.valid);
    }

    #[test]
    fn test_llm_failure() {
        let result = validate_outcome(&ValidationContext {
            user_input: "test", intent: IntentCategory::Question,
            response: "Some response text here", exec_results: &[], plan: None,
            llm_ok: false,
        });
        assert!(!result.valid);
        assert_eq!(result.suggested_outcome, SuggestedOutcome::Failure);
    }

    #[test]
    fn test_code_task_with_code() {
        let result = validate_outcome(&ValidationContext {
            user_input: "Write a function to sort a list",
            intent: IntentCategory::CodeBuild,
            response: "```rust\nfn sort(items: &mut Vec<i32>) { items.sort(); }\n```",
            exec_results: &[], plan: None, llm_ok: true,
        });
        assert!(result.valid);
        assert!(result.checks_passed.iter().any(|c| c.contains("code")));
    }

    #[test]
    fn test_exec_mixed_results() {
        let result = validate_outcome(&ValidationContext {
            user_input: "run tests",
            intent: IntentCategory::CodeBuild,
            response: "Running tests... fn test() {}",
            exec_results: &[
                ("cargo test".into(), "ok".into(), true),
                ("cargo clippy".into(), "error".into(), false),
            ],
            plan: None, llm_ok: true,
        });
        assert!(result.checks_failed.iter().any(|c| c.contains("1/2")));
    }

    #[test]
    fn test_deflection_partial() {
        let result = validate_outcome(&ValidationContext {
            user_input: "deploy the application to production server",
            intent: IntentCategory::Deploy,
            response: "I apologize but I can't deploy to production without credentials.",
            exec_results: &[], plan: None, llm_ok: true,
        });
        // Deflection detected as substance failure, but keywords overlap → partial
        assert_eq!(result.suggested_outcome, SuggestedOutcome::PartialSuccess);
        assert!(result.checks_failed.iter().any(|c| c.contains("deflection")));
    }
}
