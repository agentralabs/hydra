//! Implementation chain runner — executes multiple specs sequentially with fail-fast.

#[derive(Debug)]
pub struct ChainStep {
    pub spec_path: String,
    pub status: ChainStatus,
    pub gaps_found: usize,
    pub patches_applied: usize,
    pub error_message: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum ChainStatus {
    Pending,
    Success,
    Failed,
    Skipped,
}

pub struct ChainResult {
    pub steps: Vec<ChainStep>,
    pub total_gaps: usize,
    pub total_patches: usize,
    pub success_count: usize,
    pub failure_count: usize,
}

/// Initialize a chain of steps from spec file paths, all pending.
pub fn plan_chain(spec_paths: &[&str]) -> Vec<ChainStep> {
    spec_paths.iter().map(|&path| ChainStep {
        spec_path: path.to_string(),
        status: ChainStatus::Pending,
        gaps_found: 0,
        patches_applied: 0,
        error_message: None,
    }).collect()
}

/// Record the result of a single step execution.
pub fn record_step_result(step: &mut ChainStep, success: bool, gaps: usize, patches: usize, error: Option<String>) {
    step.gaps_found = gaps;
    step.patches_applied = patches;
    step.error_message = error;
    step.status = if success { ChainStatus::Success } else { ChainStatus::Failed };
}

/// Build a ChainResult from completed steps.
pub fn build_result(steps: Vec<ChainStep>) -> ChainResult {
    let total_gaps = steps.iter().map(|s| s.gaps_found).sum();
    let total_patches = steps.iter().map(|s| s.patches_applied).sum();
    let success_count = steps.iter().filter(|s| s.status == ChainStatus::Success).count();
    let failure_count = steps.iter().filter(|s| s.status == ChainStatus::Failed).count();
    ChainResult { steps, total_gaps, total_patches, success_count, failure_count }
}

/// Human-readable chain report.
pub fn chain_summary(result: &ChainResult) -> String {
    let mut report = String::from("## Implementation Chain Report\n\n");
    for (i, step) in result.steps.iter().enumerate() {
        let icon = match step.status {
            ChainStatus::Success => "OK",
            ChainStatus::Failed => "FAIL",
            ChainStatus::Skipped => "SKIP",
            ChainStatus::Pending => "...",
        };
        report.push_str(&format!("{}. [{}] {} — {} gaps, {} patches",
            i + 1, icon, step.spec_path, step.gaps_found, step.patches_applied));
        if let Some(ref err) = step.error_message {
            report.push_str(&format!(" ({})", err));
        }
        report.push('\n');
    }
    report.push_str(&format!("\nTotal: {} gaps, {} patches, {}/{} succeeded\n",
        result.total_gaps, result.total_patches, result.success_count, result.steps.len()));
    report
}

/// Check if the chain should continue (fail-fast: stop on first failure).
pub fn should_continue(result: &ChainResult) -> bool {
    result.failure_count == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_chain() {
        let steps = plan_chain(&["spec-a.md", "spec-b.md"]);
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].status, ChainStatus::Pending);
    }

    #[test]
    fn test_record_and_build() {
        let mut steps = plan_chain(&["a.md", "b.md"]);
        record_step_result(&mut steps[0], true, 3, 2, None);
        record_step_result(&mut steps[1], false, 1, 0, Some("LLM timeout".into()));
        let result = build_result(steps);
        assert_eq!(result.success_count, 1);
        assert_eq!(result.failure_count, 1);
        assert_eq!(result.total_gaps, 4);
        assert!(!should_continue(&result));
    }

    #[test]
    fn test_chain_summary() {
        let mut steps = plan_chain(&["spec.md"]);
        record_step_result(&mut steps[0], true, 2, 2, None);
        let result = build_result(steps);
        let summary = chain_summary(&result);
        assert!(summary.contains("[OK]"));
        assert!(summary.contains("1/1 succeeded"));
    }

    #[test]
    fn test_should_continue_all_success() {
        let mut steps = plan_chain(&["a.md", "b.md"]);
        record_step_result(&mut steps[0], true, 1, 1, None);
        record_step_result(&mut steps[1], true, 2, 2, None);
        let result = build_result(steps);
        assert!(should_continue(&result));
    }
}
