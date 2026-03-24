//! Shared types for swarm browser operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A high-level research goal from the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmGoal {
    pub id: Uuid,
    pub description: String,
    pub max_workers: usize,
    pub created_at: DateTime<Utc>,
}

impl SwarmGoal {
    pub fn new(description: impl Into<String>, max_workers: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            max_workers: max_workers.min(crate::constants::MAX_POOL_SIZE),
            created_at: Utc::now(),
        }
    }
}

/// A decomposed sub-task assigned to one browser worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTask {
    pub id: Uuid,
    pub parent_goal_id: Uuid,
    pub query: String,
    pub task_type: SwarmTaskType,
}

impl SwarmTask {
    pub fn new(parent_goal_id: Uuid, query: impl Into<String>, task_type: SwarmTaskType) -> Self {
        Self { id: Uuid::new_v4(), parent_goal_id, query: query.into(), task_type }
    }
}

/// Type of sub-task for a browser worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwarmTaskType {
    WebSearch,
    DeepRead { url: String },
    YouTubeTranscript { video_url: String },
    DocumentExtract { url: String },
}

/// Result from a single browser worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    pub task_id: Uuid,
    pub worker_id: Uuid,
    pub content: String,
    pub source_url: String,
    pub confidence: f64,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// Extracted YouTube transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTranscript {
    pub video_url: String,
    pub title: String,
    pub segments: Vec<TranscriptSegment>,
    pub full_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub timestamp_secs: f64,
    pub text: String,
}

/// Merged knowledge from all workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedKnowledge {
    pub summary: String,
    pub sources: Vec<String>,
    pub confidence: f64,
    pub worker_count: usize,
}

/// The full response from a swarm operation.
#[derive(Debug, Clone)]
pub struct SwarmResponse {
    pub goal: SwarmGoal,
    pub results: Vec<WorkerResult>,
    pub merged: MergedKnowledge,
    pub consensus_reached: bool,
    pub genome_entries_created: usize,
    pub total_duration_ms: u64,
}

impl SwarmResponse {
    pub fn format_display(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Swarm Research: {}\n\n", self.goal.description));
        out.push_str(&self.merged.summary);
        out.push_str("\n\n---\n");
        out.push_str(&format!("Sources: {}\n", self.merged.sources.join(", ")));
        out.push_str(&format!(
            "Workers: {} | Consensus: {} | Confidence: {:.0}% | {}ms | {} genome entries\n",
            self.results.len(),
            if self.consensus_reached { "yes" } else { "no" },
            self.merged.confidence * 100.0,
            self.total_duration_ms,
            self.genome_entries_created,
        ));
        out
    }
}

/// Progress updates sent to the TUI.
#[derive(Debug, Clone)]
pub enum SwarmUpdate {
    Decomposing { goal: String },
    WorkerSpawned { worker_id: Uuid, query: String },
    WorkerProgress { worker_id: Uuid, status: String },
    WorkerComplete { worker_id: Uuid, preview: String },
    WorkerFailed { worker_id: Uuid, error: String },
    Merging { count: usize },
    Complete(SwarmResponse),
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swarm_goal_caps_workers() {
        let goal = SwarmGoal::new("test", 100);
        assert!(goal.max_workers <= crate::constants::MAX_POOL_SIZE);
    }

    #[test]
    fn swarm_task_creates_with_id() {
        let goal_id = Uuid::new_v4();
        let task = SwarmTask::new(goal_id, "test query", SwarmTaskType::WebSearch);
        assert_eq!(task.parent_goal_id, goal_id);
    }
}
