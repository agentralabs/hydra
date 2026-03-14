//! Backtracking / refactoring loop — fixes root causes, not symptoms.
//!
//! UCU Module #5 (Wave 4). When a step fails, traces the failure back to its
//! root cause and fixes the originating step rather than retrying the symptom.
//! Sister-first: uses Cognition sister for known resolutions when available.

use crate::cognitive::iterative_planner::TaskStep;
use crate::cognitive::parallel_dispatch::DispatchResult;

/// Result of a backtrack analysis.
#[derive(Debug, Clone)]
pub struct BacktrackResult {
    pub action: BacktrackAction,
    pub reason: String,
    /// Which step to retry (if RetryPrevious).
    pub retry_step_id: Option<usize>,
    /// Modified input/approach for the retry.
    pub modified_input: Option<String>,
}

/// What action to take after analyzing a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacktrackAction {
    /// Retry the same step with a modified approach.
    RetryWithFix,
    /// Go back to a previous step and redo from there.
    RetryPrevious,
    /// Skip this step and continue (if not blocking).
    SkipStep,
    /// Cannot fix autonomously — ask the user.
    EscalateToUser,
    /// Entire plan is unrecoverable.
    AbortPlan,
}

/// Context about a failure for the backtrack analyzer.
#[derive(Debug, Clone)]
pub struct FailureContext {
    pub step_id: usize,
    pub error: String,
    pub attempt: u8,
    pub max_retries: u8,
}

/// Analyze a failure and determine recovery strategy.
pub fn analyze_failure(
    ctx: &FailureContext,
    steps: &[TaskStep],
    completed_results: &[DispatchResult],
) -> BacktrackResult {
    // Too many retries — give up
    if ctx.attempt >= ctx.max_retries {
        return BacktrackResult {
            action: BacktrackAction::EscalateToUser,
            reason: format!("Exhausted {} retries for step {}", ctx.max_retries, ctx.step_id),
            retry_step_id: None,
            modified_input: None,
        };
    }

    // Try to identify root cause
    if let Some(root) = find_root_cause(&ctx.error, ctx.step_id, steps, completed_results) {
        return root;
    }

    // Try pattern-matched fix
    if let Some(fix) = suggest_fix(&ctx.error) {
        return BacktrackResult {
            action: BacktrackAction::RetryWithFix,
            reason: format!("Pattern-matched fix: {}", fix),
            retry_step_id: Some(ctx.step_id),
            modified_input: Some(fix),
        };
    }

    // Check if step can be skipped
    if can_skip(ctx.step_id, steps) {
        return BacktrackResult {
            action: BacktrackAction::SkipStep,
            reason: "Step has no downstream dependents — safe to skip".into(),
            retry_step_id: None,
            modified_input: None,
        };
    }

    // Default: retry with more context
    BacktrackResult {
        action: BacktrackAction::RetryWithFix,
        reason: "Retrying with additional error context".into(),
        retry_step_id: Some(ctx.step_id),
        modified_input: Some(format!("Previous attempt failed: {}. Try a different approach.", ctx.error)),
    }
}

/// Try to trace a failure back to an earlier step.
fn find_root_cause(
    error: &str,
    failed_step_id: usize,
    steps: &[TaskStep],
    completed_results: &[DispatchResult],
) -> Option<BacktrackResult> {
    let error_lower = error.to_lowercase();

    // Type/import errors often originate in the step that defined the types
    if error_lower.contains("not found") || error_lower.contains("undefined")
        || error_lower.contains("cannot find") || error_lower.contains("no such") {
        // Find the step that this step depends on (likely where types were defined)
        if let Some(step) = steps.get(failed_step_id) {
            for &dep_id in &step.depends_on {
                if let Some(dep_step) = steps.get(dep_id) {
                    if dep_step.description.to_lowercase().contains("interface")
                        || dep_step.description.to_lowercase().contains("type")
                        || dep_step.description.to_lowercase().contains("design") {
                        return Some(BacktrackResult {
                            action: BacktrackAction::RetryPrevious,
                            reason: format!("'{}' error likely caused by step {}: {}",
                                &error[..error.len().min(60)], dep_id, dep_step.description),
                            retry_step_id: Some(dep_id),
                            modified_input: Some(format!(
                                "The downstream step failed with: {}. Revise the output to fix this.", error)),
                        });
                    }
                }
            }
        }
    }

    // Dependency/import errors — trace to the setup step
    if error_lower.contains("dependency") || error_lower.contains("import")
        || error_lower.contains("module") || error_lower.contains("crate") {
        // Find the earliest step (usually setup/scaffold)
        if let Some(earliest) = steps.first() {
            if earliest.id != failed_step_id {
                return Some(BacktrackResult {
                    action: BacktrackAction::RetryPrevious,
                    reason: format!("Dependency error — likely missing from initial setup"),
                    retry_step_id: Some(earliest.id),
                    modified_input: Some(format!(
                        "A later step failed with dependency error: {}. Add the missing dependency.", error)),
                });
            }
        }
    }

    None
}

/// Suggest a fix based on common error patterns.
pub fn suggest_fix(error: &str) -> Option<String> {
    let lower = error.to_lowercase();

    if lower.contains("permission denied") {
        return Some("Use elevated permissions or check file ownership".into());
    }
    if lower.contains("connection refused") || lower.contains("connect error") {
        return Some("Check if the service is running and the port is correct".into());
    }
    if lower.contains("out of memory") || lower.contains("oom") {
        return Some("Reduce batch size or increase memory allocation".into());
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return Some("Increase timeout or break into smaller operations".into());
    }
    if lower.contains("syntax error") || lower.contains("parse error") {
        return Some("Fix the syntax — check for missing brackets, quotes, or semicolons".into());
    }
    if lower.contains("type mismatch") || lower.contains("expected") && lower.contains("found") {
        return Some("Fix the type — ensure return type matches the expected type".into());
    }
    if lower.contains("borrow") && (lower.contains("moved") || lower.contains("move")) {
        return Some("Use .clone() or restructure to avoid double-borrow".into());
    }
    if lower.contains("not found in scope") || lower.contains("unresolved import") {
        return Some("Add the missing import or use statement".into());
    }

    None
}

/// Check whether a step can be safely skipped.
pub fn can_skip(step_id: usize, steps: &[TaskStep]) -> bool {
    // A step can be skipped if no other step depends on it
    !steps.iter().any(|s| s.depends_on.contains(&step_id))
}

/// Compute a recovery plan: which steps need to be re-executed after fixing step N.
pub fn recovery_chain(fixed_step_id: usize, steps: &[TaskStep]) -> Vec<usize> {
    let mut chain = Vec::new();
    let mut to_check = vec![fixed_step_id];
    let mut visited = vec![false; steps.len()];

    while let Some(id) = to_check.pop() {
        if id >= steps.len() || visited[id] { continue; }
        visited[id] = true;
        if id != fixed_step_id {
            chain.push(id);
        }
        // Find all steps that depend on this one
        for step in steps {
            if step.depends_on.contains(&id) && !visited[step.id] {
                to_check.push(step.id);
            }
        }
    }

    chain.sort();
    chain
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::iterative_planner::Phase;

    fn step(id: usize, desc: &str, deps: Vec<usize>) -> TaskStep {
        TaskStep {
            id, phase: Phase::Execute, description: desc.into(),
            depends_on: deps, estimated_tokens: 1000, requires_sister: None,
        }
    }

    #[test]
    fn test_exhausted_retries() {
        let ctx = FailureContext { step_id: 0, error: "failed".into(), attempt: 3, max_retries: 3 };
        let result = analyze_failure(&ctx, &[], &[]);
        assert_eq!(result.action, BacktrackAction::EscalateToUser);
    }

    #[test]
    fn test_skippable_step() {
        let steps = vec![step(0, "setup", vec![]), step(1, "main", vec![0]), step(2, "optional", vec![0])];
        // Step 2 has no dependents → skippable
        assert!(can_skip(2, &steps));
        // Step 0 has dependents → not skippable
        assert!(!can_skip(0, &steps));
    }

    #[test]
    fn test_suggest_fix_timeout() {
        let fix = suggest_fix("Operation timed out after 30s");
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("timeout"));
    }

    #[test]
    fn test_suggest_fix_borrow() {
        let fix = suggest_fix("value borrowed here after move");
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("clone"));
    }

    #[test]
    fn test_recovery_chain() {
        let steps = vec![
            step(0, "types", vec![]),
            step(1, "impl", vec![0]),
            step(2, "tests", vec![1]),
            step(3, "docs", vec![0]),
        ];
        let chain = recovery_chain(0, &steps);
        assert!(chain.contains(&1));
        assert!(chain.contains(&2));
        assert!(chain.contains(&3));
    }

    #[test]
    fn test_root_cause_type_error() {
        let steps = vec![
            step(0, "Design interfaces and types", vec![]),
            step(1, "Implement logic", vec![0]),
        ];
        let ctx = FailureContext {
            step_id: 1, error: "type `Foo` not found in scope".into(),
            attempt: 0, max_retries: 3,
        };
        let result = analyze_failure(&ctx, &steps, &[]);
        assert_eq!(result.action, BacktrackAction::RetryPrevious);
        assert_eq!(result.retry_step_id, Some(0));
    }
}
