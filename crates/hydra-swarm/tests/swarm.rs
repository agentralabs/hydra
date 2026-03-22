//! Integration tests for hydra-swarm.

use hydra_fleet::agent::AgentSpecialization;
use hydra_fleet::assignment::AssignmentStrategy;
use hydra_fleet::registry::FleetRegistry;
use hydra_fleet::result::{AgentResult, ResultOutcome};
use hydra_fleet::task::{FleetTask, TaskType};
use hydra_swarm::consensus::AgentAnswer;
use hydra_swarm::emergence::EmergenceEntry;
use hydra_swarm::health::SwarmHealthLevel;
use hydra_swarm::swarm::Swarm;
use hydra_trust::TrustTier;
use uuid::Uuid;

fn make_registry_with_agents(n: usize) -> FleetRegistry {
    let mut reg = FleetRegistry::new();
    for i in 0..n {
        reg.spawn(
            format!("agent-{i}"),
            AgentSpecialization::Generalist,
            "test-root",
            0.5,
            TrustTier::Silver,
        )
        .expect("spawn should succeed");
    }
    reg
}

#[test]
fn consensus_with_identical_answers() {
    let answers = vec![
        AgentAnswer::from_text(Uuid::new_v4(), "the result is correct"),
        AgentAnswer::from_text(Uuid::new_v4(), "the result is correct"),
        AgentAnswer::from_text(Uuid::new_v4(), "the result is correct"),
    ];
    let swarm = Swarm::new(FleetRegistry::new());
    let signal = swarm.evaluate_consensus(&answers).unwrap();
    assert!(signal.reached);
    assert!(signal.is_strong());
    assert_eq!(signal.agreeing_count, 3);
}

#[test]
fn no_consensus_with_different_answers() {
    let answers = vec![
        AgentAnswer::from_text(Uuid::new_v4(), "alpha beta gamma"),
        AgentAnswer::from_text(Uuid::new_v4(), "delta epsilon zeta"),
        AgentAnswer::from_text(Uuid::new_v4(), "eta theta iota"),
    ];
    let swarm = Swarm::new(FleetRegistry::new());
    let signal = swarm.evaluate_consensus(&answers).unwrap();
    assert!(!signal.reached);
}

#[test]
fn insufficient_agents_for_consensus() {
    let answers = vec![AgentAnswer::from_text(Uuid::new_v4(), "alone")];
    let swarm = Swarm::new(FleetRegistry::new());
    let result = swarm.evaluate_consensus(&answers);
    assert!(result.is_err());
}

#[test]
fn emergence_append_only() {
    let mut swarm = Swarm::new(FleetRegistry::new());
    let e1 = EmergenceEntry::new("pattern one", 3, 0.85);
    let e2 = EmergenceEntry::new("pattern two", 4, 0.90);
    assert!(swarm.record_emergence(e1));
    assert_eq!(swarm.emergence_count(), 1);
    assert!(swarm.record_emergence(e2));
    assert_eq!(swarm.emergence_count(), 2);
}

#[test]
fn healthy_swarm_has_positive_lyapunov() {
    let reg = make_registry_with_agents(4);
    let mut swarm = Swarm::new(reg);
    let delta = swarm.lyapunov_delta();
    assert!(delta > 0.0);
    let health = swarm.health();
    assert_eq!(health.level, SwarmHealthLevel::Healthy);
}

#[test]
fn full_lifecycle_integration() {
    let mut reg = make_registry_with_agents(3);

    // Assign and complete a task for each agent
    let agents: Vec<Uuid> = reg.agents().iter().map(|a| a.id).collect();
    for &aid in &agents {
        let task = FleetTask::new(TaskType::CodeReview, "review code", 5, "root");
        let tid = task.id;
        reg.assign_task(task, &AssignmentStrategy::FirstAvailable)
            .expect("assign ok");
        let result =
            AgentResult::new(aid, tid, ResultOutcome::Success, "approved").expect("result ok");
        let receipt = reg.submit_result(result).expect("submit ok");
        assert_eq!(receipt.outcome, ResultOutcome::Success);
    }

    // Build swarm and check consensus
    let answers: Vec<AgentAnswer> = agents
        .iter()
        .map(|&id| AgentAnswer::from_text(id, "code is clean and approved"))
        .collect();
    let mut swarm = Swarm::new(reg);
    let signal = swarm.evaluate_consensus(&answers).unwrap();
    assert!(signal.reached);

    // Health check
    let health = swarm.health();
    assert_eq!(health.level, SwarmHealthLevel::Healthy);
    assert!(swarm.lyapunov_delta() > 0.0);
}
