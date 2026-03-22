//! Integration tests for hydra-fleet.

use hydra_fleet::agent::AgentSpecialization;
use hydra_fleet::assignment::AssignmentStrategy;
use hydra_fleet::registry::FleetRegistry;
use hydra_fleet::result::{AgentResult, ResultOutcome};
use hydra_fleet::task::{FleetTask, TaskType};
use hydra_trust::TrustTier;

fn spawn_default(registry: &mut FleetRegistry, name: &str) -> uuid::Uuid {
    registry
        .spawn(
            name,
            AgentSpecialization::Generalist,
            "test-causal-root",
            0.5,
            TrustTier::Silver,
        )
        .expect("spawn should succeed")
}

#[test]
fn spawn_and_count() {
    let mut reg = FleetRegistry::new();
    let id = spawn_default(&mut reg, "agent-1");
    assert_eq!(reg.agent_count(), 1);
    assert!(reg.get_agent(&id).is_some());
}

#[test]
fn task_lifecycle() {
    let mut reg = FleetRegistry::new();
    let agent_id = spawn_default(&mut reg, "worker");

    let task = FleetTask::new(TaskType::CodeAnalysis, "analyse main.rs", 5, "test-root");
    let task_id = task.id;

    let assigned = reg
        .assign_task(task, &AssignmentStrategy::FirstAvailable)
        .expect("assign should succeed");
    assert_eq!(assigned, agent_id);

    let result = AgentResult::new(agent_id, task_id, ResultOutcome::Success, "done")
        .expect("result should be valid");

    let receipt = reg.submit_result(result).expect("submit should succeed");
    assert_eq!(receipt.agent_id, agent_id);
}

#[test]
fn quarantined_agent_rejects_tasks() {
    let mut reg = FleetRegistry::new();
    let agent_id = spawn_default(&mut reg, "quarantine-test");
    reg.quarantine(&agent_id).expect("quarantine should work");

    let task = FleetTask::new(TaskType::Testing, "run tests", 3, "root");
    // No idle agents available, so assign should fail
    let result = reg.assign_task(task, &AssignmentStrategy::FirstAvailable);
    assert!(result.is_err());
}

#[test]
fn receipt_before_consumption() {
    let mut reg = FleetRegistry::new();
    let agent_id = spawn_default(&mut reg, "receipt-test");

    let task = FleetTask::new(TaskType::CodeReview, "review", 1, "root");
    let task_id = task.id;
    reg.assign_task(task, &AssignmentStrategy::FirstAvailable)
        .expect("assign ok");

    let result =
        AgentResult::new(agent_id, task_id, ResultOutcome::Success, "lgtm").expect("result ok");
    let receipt = reg.submit_result(result).expect("submit ok");

    // Receipt was issued — verify it exists in the registry
    assert!(!reg.receipts().is_empty());
    assert_eq!(reg.receipts()[0].receipt_id, receipt.receipt_id);
}

#[test]
fn capability_match_assignment() {
    let mut reg = FleetRegistry::new();
    reg.spawn(
        "analyst",
        AgentSpecialization::Analyst,
        "root",
        0.5,
        TrustTier::Silver,
    )
    .expect("spawn analyst");
    reg.spawn(
        "tester",
        AgentSpecialization::Tester,
        "root",
        0.5,
        TrustTier::Silver,
    )
    .expect("spawn tester");

    let task = FleetTask::new(TaskType::CodeAnalysis, "analyse", 5, "root");
    let assigned = reg
        .assign_task(task, &AssignmentStrategy::CapabilityMatch)
        .expect("assign should match analyst");

    let agent = reg.get_agent(&assigned).expect("agent exists");
    assert_eq!(agent.specialization, AgentSpecialization::Analyst);
}

#[test]
fn priority_clamped_at_ten() {
    let task = FleetTask::new(TaskType::Debugging, "debug", 255, "root");
    assert_eq!(task.priority, 10);
}

#[test]
fn low_trust_spawn_rejected() {
    let mut reg = FleetRegistry::new();
    let result = reg.spawn(
        "low-trust",
        AgentSpecialization::Generalist,
        "root",
        0.1, // below SPAWN_MIN_TRUST_SCORE
        TrustTier::Bronze,
    );
    assert!(result.is_err());
}
