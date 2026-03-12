//! Result aggregator — collect and merge results from agents.

use super::agent::TaskResult;

/// Aggregated report from multiple agent results.
#[derive(Debug, Clone)]
pub struct AggregatedReport {
    pub total_agents: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub duration_ms: u64,
    pub per_agent: Vec<TaskResult>,
    pub summary: String,
}

impl AggregatedReport {
    /// Format for display.
    pub fn display(&self) -> String {
        let mut out = format!(
            "Swarm Results ({}/{} succeeded, {} failed, {}ms total)\n\n",
            self.succeeded, self.total_agents, self.failed, self.duration_ms,
        );
        for result in &self.per_agent {
            let icon = if result.success { "+" } else { "x" };
            out.push_str(&format!(
                "  {} [{}] {} ({}ms, quality: {:.0}%)\n",
                icon,
                &result.agent_id[..8.min(result.agent_id.len())],
                if result.success { "OK" } else {
                    result.error.as_deref().unwrap_or("failed")
                },
                result.duration_ms,
                result.quality_score * 100.0,
            ));
        }
        if !self.summary.is_empty() {
            out.push_str(&format!("\nSummary: {}\n", self.summary));
        }
        out
    }
}

/// Aggregates results from multiple agents.
pub struct ResultAggregator;

impl ResultAggregator {
    pub fn new() -> Self {
        Self
    }

    /// Merge results from multiple agents into one report.
    pub fn aggregate(&self, results: &[TaskResult]) -> AggregatedReport {
        let succeeded = results.iter().filter(|r| r.success).count();
        let failed = results.iter().filter(|r| !r.success).count();
        let duration_ms = results.iter()
            .map(|r| r.duration_ms)
            .max()
            .unwrap_or(0);

        let summary = self.generate_summary(results);

        AggregatedReport {
            total_agents: results.len(),
            succeeded,
            failed,
            duration_ms,
            per_agent: results.to_vec(),
            summary,
        }
    }

    /// Pick the best result from parallel exploration.
    pub fn pick_best<'a>(&self, results: &'a [TaskResult]) -> Option<&'a TaskResult> {
        results.iter()
            .filter(|r| r.success)
            .max_by(|a, b| {
                a.quality_score
                    .partial_cmp(&b.quality_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Generate a text summary from results.
    fn generate_summary(&self, results: &[TaskResult]) -> String {
        if results.is_empty() {
            return "No results to aggregate.".into();
        }

        let succeeded = results.iter().filter(|r| r.success).count();
        let total = results.len();
        let avg_quality = if total > 0 {
            results.iter().map(|r| r.quality_score).sum::<f64>() / total as f64
        } else {
            0.0
        };
        let avg_duration = if total > 0 {
            results.iter().map(|r| r.duration_ms).sum::<u64>() / total as u64
        } else {
            0
        };

        if succeeded == total {
            format!(
                "All {} agents completed successfully. Avg quality: {:.0}%, avg duration: {}ms.",
                total, avg_quality * 100.0, avg_duration,
            )
        } else if succeeded == 0 {
            format!("All {} agents failed.", total)
        } else {
            format!(
                "{}/{} agents succeeded ({} failed). Avg quality: {:.0}%, avg duration: {}ms.",
                succeeded, total, total - succeeded, avg_quality * 100.0, avg_duration,
            )
        }
    }
}

impl Default for ResultAggregator {
    fn default() -> Self {
        Self::new()
    }
}
