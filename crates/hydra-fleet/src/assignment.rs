//! Task assignment strategies for fleet agents.

use crate::agent::{AgentSpecialization, FleetAgent, FleetAgentState};
use crate::task::TaskType;

/// Strategy for selecting which agent receives a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentStrategy {
    /// Assign to the first idle agent found.
    FirstAvailable,
    /// Assign to the agent with the highest success rate.
    BestPerformer,
    /// Assign to an agent whose specialization matches the task.
    CapabilityMatch,
}

/// Find the best agent for a task from a list of agents.
///
/// Returns the index of the chosen agent, or None if no suitable agent exists.
pub fn find_agent(
    agents: &[FleetAgent],
    task_type: &TaskType,
    strategy: &AssignmentStrategy,
) -> Option<usize> {
    match strategy {
        AssignmentStrategy::FirstAvailable => find_first_available(agents),
        AssignmentStrategy::BestPerformer => find_best_performer(agents),
        AssignmentStrategy::CapabilityMatch => {
            find_capability_match(agents, task_type).or_else(|| find_first_available(agents))
        }
    }
}

/// Return the preferred specialization for a given task type.
pub fn preferred_specialization(task_type: &TaskType) -> AgentSpecialization {
    match task_type {
        TaskType::CodeAnalysis => AgentSpecialization::Analyst,
        TaskType::CodeGeneration => AgentSpecialization::Generator,
        TaskType::CodeReview => AgentSpecialization::Reviewer,
        TaskType::SecurityAudit => AgentSpecialization::SecurityAuditor,
        TaskType::Testing => AgentSpecialization::Tester,
        TaskType::Documentation => AgentSpecialization::Documenter,
        TaskType::Refactoring => AgentSpecialization::Generalist,
        TaskType::Debugging => AgentSpecialization::Debugger,
    }
}

/// Find the first idle agent.
fn find_first_available(agents: &[FleetAgent]) -> Option<usize> {
    agents.iter().position(|a| a.state == FleetAgentState::Idle)
}

/// Find the idle agent with the highest success rate.
fn find_best_performer(agents: &[FleetAgent]) -> Option<usize> {
    agents
        .iter()
        .enumerate()
        .filter(|(_, a)| a.state == FleetAgentState::Idle)
        .max_by(|(_, a), (_, b)| {
            a.success_rate()
                .partial_cmp(&b.success_rate())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
}

/// Find an idle agent whose specialization matches the task.
fn find_capability_match(agents: &[FleetAgent], task_type: &TaskType) -> Option<usize> {
    let preferred = preferred_specialization(task_type);
    agents
        .iter()
        .position(|a| a.state == FleetAgentState::Idle && a.specialization == preferred)
}
