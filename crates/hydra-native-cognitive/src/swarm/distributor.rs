//! Task distributor — decompose goals into subtasks and assign to agents.

use super::agent::{AgentInstance, AgentTask, Assignment};

/// Distributes tasks across a swarm of agents.
pub struct TaskDistributor;

impl TaskDistributor {
    pub fn new() -> Self {
        Self
    }

    /// Decompose a goal into subtasks for parallel execution.
    ///
    /// Uses simple heuristics — for LLM-powered decomposition,
    /// the cognitive loop handles it before reaching the distributor.
    pub fn decompose(&self, goal: &str, agent_count: usize) -> Vec<AgentTask> {
        if agent_count == 0 {
            return vec![];
        }

        // Simple decomposition: if goal mentions specific patterns, split accordingly
        let lower = goal.to_lowercase();

        // Pattern: "test X with N agents" → N identical test tasks
        if lower.contains("test") {
            return self.duplicate_task(goal, agent_count);
        }

        // Pattern: "review" or "audit" → split by area
        if lower.contains("review") || lower.contains("audit") {
            let areas = ["security", "performance", "correctness", "style", "documentation"];
            return areas.iter()
                .take(agent_count)
                .enumerate()
                .map(|(i, area)| AgentTask {
                    id: format!("task-{}-{}", &uuid_short(), i),
                    description: format!("{} — focus on {}", goal, area),
                    required_skills: vec![area.to_string()],
                    priority: 5,
                    timeout_secs: 300,
                })
                .collect();
        }

        // Pattern: "scan" or "search" → split by directory/module
        if lower.contains("scan") || lower.contains("search") {
            return (0..agent_count)
                .map(|i| AgentTask {
                    id: format!("task-{}-{}", &uuid_short(), i),
                    description: format!("{} — partition {}/{}", goal, i + 1, agent_count),
                    required_skills: vec![],
                    priority: 5,
                    timeout_secs: 300,
                })
                .collect();
        }

        // Default: single task if can't decompose, or duplicate for parallel
        if agent_count == 1 {
            vec![AgentTask {
                id: format!("task-{}", &uuid_short()),
                description: goal.to_string(),
                required_skills: vec![],
                priority: 5,
                timeout_secs: 300,
            }]
        } else {
            self.duplicate_task(goal, agent_count)
        }
    }

    /// Assign tasks to agents based on skills and availability.
    pub fn assign(
        &self,
        tasks: &[AgentTask],
        agents: &[AgentInstance],
    ) -> Vec<Assignment> {
        let mut assignments = Vec::new();
        let available: Vec<&AgentInstance> = agents.iter()
            .filter(|a| a.can_handle(&AgentTask {
                id: String::new(),
                description: String::new(),
                required_skills: vec![],
                priority: 0,
                timeout_secs: 0,
            }))
            .collect();

        for (i, task) in tasks.iter().enumerate() {
            // Find best matching agent
            let agent = if task.required_skills.is_empty() {
                // Round-robin for generic tasks
                available.get(i % available.len().max(1))
            } else {
                // Skill-matched
                available.iter()
                    .find(|a| a.can_handle(task))
                    .or_else(|| available.get(i % available.len().max(1)))
            };

            if let Some(agent) = agent {
                assignments.push(Assignment {
                    agent_id: agent.id.clone(),
                    task: task.clone(),
                });
            }
        }

        assignments
    }

    /// Duplicate a task for N agents (parallel exploration).
    pub fn duplicate_task(&self, description: &str, count: usize) -> Vec<AgentTask> {
        (0..count)
            .map(|i| AgentTask {
                id: format!("task-{}-{}", &uuid_short(), i),
                description: format!("{} (agent {}/{})", description, i + 1, count),
                required_skills: vec![],
                priority: 5,
                timeout_secs: 300,
            })
            .collect()
    }
}

impl Default for TaskDistributor {
    fn default() -> Self {
        Self::new()
    }
}

fn uuid_short() -> String {
    uuid::Uuid::new_v4().to_string()[..8].to_string()
}
