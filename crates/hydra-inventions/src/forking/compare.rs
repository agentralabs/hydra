//! OutcomeComparator — compare results from forked branches.

use serde::{Deserialize, Serialize};

use super::parallel::BranchResult;

/// Result of comparing branch outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub best_branch: String,
    pub best_name: String,
    pub score: f64,
    pub rankings: Vec<BranchRanking>,
    pub unanimous_success: bool,
}

/// Ranking for a single branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchRanking {
    pub branch_id: String,
    pub branch_name: String,
    pub rank: usize,
    pub score: f64,
    pub success: bool,
}

/// Compares outcomes from parallel branches
pub struct OutcomeComparator {
    /// Weight for success (vs speed) in scoring
    success_weight: f64,
    speed_weight: f64,
    resource_weight: f64,
}

impl OutcomeComparator {
    pub fn new() -> Self {
        Self {
            success_weight: 0.6,
            speed_weight: 0.25,
            resource_weight: 0.15,
        }
    }

    pub fn with_weights(success: f64, speed: f64, resources: f64) -> Self {
        let total = success + speed + resources;
        Self {
            success_weight: success / total,
            speed_weight: speed / total,
            resource_weight: resources / total,
        }
    }

    /// Score a branch result
    pub fn score(&self, result: &BranchResult) -> f64 {
        let success_score = if result.success { 1.0 } else { 0.0 };
        let speed_score = 1.0 / (1.0 + result.duration_ms as f64 / 1000.0);
        let resource_score = 1.0 / (1.0 + result.resources_used);

        success_score * self.success_weight
            + speed_score * self.speed_weight
            + resource_score * self.resource_weight
    }

    /// Compare all branch results and rank them
    pub fn compare(&self, results: &[BranchResult]) -> ComparisonResult {
        let mut scored: Vec<_> = results.iter().map(|r| (self.score(r), r)).collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let rankings: Vec<BranchRanking> = scored
            .iter()
            .enumerate()
            .map(|(i, (score, result))| BranchRanking {
                branch_id: result.branch_id.clone(),
                branch_name: result.branch_name.clone(),
                rank: i + 1,
                score: *score,
                success: result.success,
            })
            .collect();

        let best = scored.first().map(|(_, r)| r);
        let unanimous = results.iter().all(|r| r.success);

        ComparisonResult {
            best_branch: best.map(|r| r.branch_id.clone()).unwrap_or_default(),
            best_name: best.map(|r| r.branch_name.clone()).unwrap_or_default(),
            score: scored.first().map(|(s, _)| *s).unwrap_or(0.0),
            rankings,
            unanimous_success: unanimous,
        }
    }
}

impl Default for OutcomeComparator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outcome_comparison() {
        let comparator = OutcomeComparator::new();
        let results = vec![
            BranchResult {
                branch_id: "b1".into(),
                branch_name: "fast".into(),
                success: true,
                output: serde_json::json!({}),
                duration_ms: 50,
                resources_used: 0.1,
            },
            BranchResult {
                branch_id: "b2".into(),
                branch_name: "slow".into(),
                success: true,
                output: serde_json::json!({}),
                duration_ms: 500,
                resources_used: 0.5,
            },
            BranchResult {
                branch_id: "b3".into(),
                branch_name: "failed".into(),
                success: false,
                output: serde_json::json!({}),
                duration_ms: 100,
                resources_used: 0.2,
            },
        ];

        let comparison = comparator.compare(&results);
        assert_eq!(comparison.best_name, "fast");
        assert_eq!(comparison.rankings.len(), 3);
        assert_eq!(comparison.rankings[0].rank, 1);
        assert!(!comparison.unanimous_success);
    }

    #[test]
    fn test_best_path_selection() {
        let comparator = OutcomeComparator::new();
        let results = vec![
            BranchResult {
                branch_id: "a".into(),
                branch_name: "approach_a".into(),
                success: true,
                output: serde_json::json!({}),
                duration_ms: 200,
                resources_used: 0.3,
            },
            BranchResult {
                branch_id: "b".into(),
                branch_name: "approach_b".into(),
                success: true,
                output: serde_json::json!({}),
                duration_ms: 100,
                resources_used: 0.1,
            },
        ];

        let comparison = comparator.compare(&results);
        // approach_b is faster and uses fewer resources
        assert_eq!(comparison.best_name, "approach_b");
        assert!(comparison.unanimous_success);
    }
}
