//! V3 Evaluator — scores tests with equal weight per category + capability scorecard.
//! Each test gets a 0–100% capability score with breakdown.

use super::bank::{V3Category, V3Test};
use super::runner::V3Result;

/// Per-test capability score for the scorecard.
#[derive(Debug, Clone)]
pub struct CapabilityScore {
    pub test_id: String,
    pub name: String,
    pub category: String,
    pub percentage: f64,     // 0–100
    pub breakdown: String,   // How the % was computed
    pub output_preview: String, // First 200 chars of actual output
    pub duration_ms: u64,
    pub tokens: usize,       // From receipt, 0 if no receipt
    pub passed: bool,
}

/// Scores for one hour of V3 testing.
#[derive(Debug, Clone)]
pub struct V3HourScores {
    pub hour: u32,
    pub results: Vec<V3Result>,
    pub category_scores: Vec<(V3Category, f64)>,
    pub overall: f64,
    pub tests_passed: usize,
    pub tests_total: usize,
    pub deployment_blocked: bool,
    pub capabilities: Vec<CapabilityScore>,
    pub total_tokens: usize,
    pub total_duration_ms: u64,
}

/// Compute category scores, overall score, and capability scorecard.
pub fn score_hour(hour: u32, results: Vec<V3Result>, tests: &[V3Test]) -> V3HourScores {
    let categories = V3Category::categories_in(tests);
    let num_cats = categories.len();
    let mut category_scores = Vec::new();
    let mut weighted_sum = 0.0;
    let mut deployment_blocked = false;

    for cat in &categories {
        let cat_results: Vec<&V3Result> = results.iter()
            .filter(|r| tests.iter().any(|t| t.id == r.test_id && t.category == *cat))
            .collect();
        let avg = if cat_results.is_empty() { 0.0 }
            else { cat_results.iter().map(|r| r.score).sum::<f64>() / cat_results.len() as f64 };
        weighted_sum += avg * cat.weight(num_cats);
        category_scores.push((*cat, avg));

        if cat.is_blocking() && cat_results.iter().any(|r| !r.passed) {
            deployment_blocked = true;
        }
    }

    // Build capability scorecard
    let capabilities: Vec<CapabilityScore> = results.iter().map(|r| {
        let test = tests.iter().find(|t| t.id == r.test_id);
        let name = test.map(|t| t.name).unwrap_or("unknown");
        let cat_label = test.map(|t| t.category.label()).unwrap_or("?");
        let tokens = r.receipt.as_ref().map(|rc| rc.tokens).unwrap_or(0);
        let preview = &r.output[..r.output.len().min(200)];
        CapabilityScore {
            test_id: r.test_id.clone(), name: name.into(),
            category: cat_label.into(), percentage: r.percentage,
            breakdown: r.breakdown.clone(), output_preview: preview.into(),
            duration_ms: r.duration_ms, tokens, passed: r.passed,
        }
    }).collect();

    let total_tokens: usize = capabilities.iter().map(|c| c.tokens).sum();
    let total_duration_ms: u64 = results.iter().map(|r| r.duration_ms).sum();
    let tests_passed = results.iter().filter(|r| r.passed).count();
    let tests_total = results.len();

    V3HourScores {
        hour, results, category_scores, overall: weighted_sum,
        tests_passed, tests_total, deployment_blocked,
        capabilities, total_tokens, total_duration_ms,
    }
}
