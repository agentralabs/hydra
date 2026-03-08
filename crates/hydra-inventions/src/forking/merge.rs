//! ResultMerger — merge best results from parallel branches.

use serde::{Deserialize, Serialize};

use super::compare::ComparisonResult;
use super::parallel::BranchResult;

/// Strategy for merging results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Take the best branch's result only
    BestOnly,
    /// Combine outputs from all successful branches
    CombineSuccessful,
    /// Take best and keep others as fallbacks
    BestWithFallbacks,
}

/// Result of merging branch outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedResult {
    pub strategy: MergeStrategy,
    pub primary: BranchResult,
    pub fallbacks: Vec<BranchResult>,
    pub combined_output: serde_json::Value,
    pub branches_used: usize,
}

/// Merges results from parallel fork execution
pub struct ResultMerger {
    strategy: MergeStrategy,
}

impl ResultMerger {
    pub fn new(strategy: MergeStrategy) -> Self {
        Self { strategy }
    }

    /// Merge branch results using the configured strategy
    pub fn merge(&self, results: &[BranchResult], comparison: &ComparisonResult) -> MergedResult {
        match self.strategy {
            MergeStrategy::BestOnly => self.merge_best_only(results, comparison),
            MergeStrategy::CombineSuccessful => self.merge_combine(results),
            MergeStrategy::BestWithFallbacks => self.merge_with_fallbacks(results, comparison),
        }
    }

    fn merge_best_only(
        &self,
        results: &[BranchResult],
        comparison: &ComparisonResult,
    ) -> MergedResult {
        let best = results
            .iter()
            .find(|r| r.branch_id == comparison.best_branch)
            .cloned()
            .unwrap_or_else(|| results[0].clone());

        MergedResult {
            strategy: MergeStrategy::BestOnly,
            combined_output: best.output.clone(),
            primary: best,
            fallbacks: Vec::new(),
            branches_used: 1,
        }
    }

    fn merge_combine(&self, results: &[BranchResult]) -> MergedResult {
        let successful: Vec<_> = results.iter().filter(|r| r.success).cloned().collect();

        let combined = serde_json::json!({
            "merged": true,
            "branches": successful.iter().map(|r| {
                serde_json::json!({
                    "branch": r.branch_name,
                    "output": r.output,
                })
            }).collect::<Vec<_>>(),
        });

        let primary = successful
            .first()
            .cloned()
            .unwrap_or_else(|| results[0].clone());
        let branches_used = successful.len();

        MergedResult {
            strategy: MergeStrategy::CombineSuccessful,
            primary,
            fallbacks: successful.into_iter().skip(1).collect(),
            combined_output: combined,
            branches_used,
        }
    }

    fn merge_with_fallbacks(
        &self,
        results: &[BranchResult],
        comparison: &ComparisonResult,
    ) -> MergedResult {
        let best = results
            .iter()
            .find(|r| r.branch_id == comparison.best_branch)
            .cloned()
            .unwrap_or_else(|| results[0].clone());

        let fallbacks: Vec<_> = comparison
            .rankings
            .iter()
            .skip(1)
            .filter_map(|ranking| {
                results
                    .iter()
                    .find(|r| r.branch_id == ranking.branch_id && r.success)
                    .cloned()
            })
            .collect();

        MergedResult {
            strategy: MergeStrategy::BestWithFallbacks,
            combined_output: best.output.clone(),
            branches_used: 1 + fallbacks.len(),
            primary: best,
            fallbacks,
        }
    }
}

impl Default for ResultMerger {
    fn default() -> Self {
        Self::new(MergeStrategy::BestOnly)
    }
}

#[cfg(test)]
mod tests {
    use super::super::compare::OutcomeComparator;
    use super::*;

    fn sample_results() -> Vec<BranchResult> {
        vec![
            BranchResult {
                branch_id: "b1".into(),
                branch_name: "fast".into(),
                success: true,
                output: serde_json::json!({"result": "fast_output"}),
                duration_ms: 50,
                resources_used: 0.1,
            },
            BranchResult {
                branch_id: "b2".into(),
                branch_name: "thorough".into(),
                success: true,
                output: serde_json::json!({"result": "thorough_output"}),
                duration_ms: 200,
                resources_used: 0.4,
            },
            BranchResult {
                branch_id: "b3".into(),
                branch_name: "failed".into(),
                success: false,
                output: serde_json::json!({}),
                duration_ms: 100,
                resources_used: 0.2,
            },
        ]
    }

    #[test]
    fn test_merge_best_only() {
        let results = sample_results();
        let comparison = OutcomeComparator::new().compare(&results);
        let merger = ResultMerger::new(MergeStrategy::BestOnly);

        let merged = merger.merge(&results, &comparison);
        assert_eq!(merged.strategy, MergeStrategy::BestOnly);
        assert_eq!(merged.branches_used, 1);
        assert!(merged.fallbacks.is_empty());
    }

    #[test]
    fn test_merge_combine() {
        let results = sample_results();
        let comparison = OutcomeComparator::new().compare(&results);
        let merger = ResultMerger::new(MergeStrategy::CombineSuccessful);

        let merged = merger.merge(&results, &comparison);
        assert_eq!(merged.strategy, MergeStrategy::CombineSuccessful);
        assert_eq!(merged.branches_used, 2); // only successful
        assert!(merged.combined_output["merged"].as_bool().unwrap());
    }
}
