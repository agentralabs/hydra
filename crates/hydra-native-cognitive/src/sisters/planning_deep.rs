//! Priority 3: Deep Planning Integration — sister-first task tracking,
//! goal lifecycle, crash recovery via Planning sister's .aplan format.
//!
//! Replaces homegrown TaskPersister with Planning sister delegation.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    /// Create a goal via Planning sister for structured task tracking.
    /// Returns a goal_id for progress tracking. Falls back to None if offline.
    pub async fn planning_create_goal(
        &self,
        name: &str,
        steps: &[String],
        deadline: Option<&str>,
    ) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_create_goal", serde_json::json!({
            "name": name,
            "steps": steps,
            "deadline": deadline,
            "source": "cognitive_loop",
        })).await.ok()?;
        result.get("goal_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Update progress on an active goal.
    pub async fn planning_update_progress(
        &self,
        goal_id: &str,
        step_index: usize,
        status: &str,
    ) {
        if let Some(planning) = &self.planning {
            if let Err(e) = planning.call_tool("planning_update_progress", serde_json::json!({
                "goal_id": goal_id,
                "step_index": step_index,
                "status": status,
            })).await {
                eprintln!("[hydra:planning] planning_update_progress FAILED: {}", e);
            }
        }
    }

    /// Complete a goal via Planning sister.
    pub async fn planning_complete_goal(&self, goal_id: &str, outcome: &str) {
        if let Some(planning) = &self.planning {
            if let Err(e) = planning.call_tool("planning_complete_goal", serde_json::json!({
                "goal_id": goal_id,
                "outcome": safe_truncate(outcome, 200),
            })).await {
                eprintln!("[hydra:planning] planning_complete_goal FAILED: {}", e);
            }
        }
    }

    /// List active goals — used for crash recovery to find interrupted tasks.
    pub async fn planning_list_active(&self) -> Option<Vec<PlanningGoal>> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_list_active", serde_json::json!({
            "include_steps": true,
        })).await.ok()?;

        let text = extract_text(&result);
        if text.is_empty() {
            return None;
        }

        // Parse goals from response
        let goals: Vec<PlanningGoal> = result.get("goals")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|g| {
                let id = g.get("goal_id")?.as_str()?.to_string();
                let name = g.get("name")?.as_str()?.to_string();
                let progress = g.get("progress")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                Some(PlanningGoal { id, name, progress })
            }).collect())
            .unwrap_or_default();

        if goals.is_empty() { None } else { Some(goals) }
    }

    /// Create a plan for project execution (/test-repo, /build, etc.).
    /// Each phase becomes a plan step tracked by Planning sister.
    pub async fn planning_create_project_plan(
        &self,
        project: &str,
        phases: &[&str],
    ) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let steps: Vec<String> = phases.iter()
            .map(|p| p.to_string())
            .collect();
        let result = planning.call_tool("planning_create_goal", serde_json::json!({
            "name": format!("Project: {}", project),
            "steps": steps,
            "source": "project_executor",
        })).await.ok()?;
        result.get("goal_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Checkpoint a project phase for crash recovery.
    pub async fn planning_checkpoint_phase(
        &self,
        goal_id: &str,
        phase: &str,
        status: &str,
        detail: &str,
    ) {
        if let Some(planning) = &self.planning {
            if let Err(e) = planning.call_tool("planning_checkpoint", serde_json::json!({
                "goal_id": goal_id,
                "phase": phase,
                "status": status,
                "detail": safe_truncate(detail, 200),
            })).await {
                eprintln!("[hydra:planning] planning_checkpoint FAILED: {}", e);
            }
        }
    }
}

/// A goal from the Planning sister.
#[derive(Debug, Clone)]
pub struct PlanningGoal {
    pub id: String,
    pub name: String,
    pub progress: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planning_goal_struct() {
        let g = PlanningGoal {
            id: "goal-001".into(),
            name: "Build auth system".into(),
            progress: 0.75,
        };
        assert_eq!(g.id, "goal-001");
        assert!(g.progress > 0.5);
    }
}
