//! Combined test harness for hydra-fleet and hydra-swarm.
//! Runs ~30 scenarios testing fleet lifecycle, swarm consensus, and integration.

use hydra_fleet::agent::AgentSpecialization;
use hydra_fleet::assignment::AssignmentStrategy;
use hydra_fleet::registry::FleetRegistry;
use hydra_fleet::result::{AgentResult, ResultOutcome, ResultReceipt};
use hydra_fleet::task::{FleetTask, TaskType};
use hydra_swarm::consensus::AgentAnswer;
use hydra_swarm::emergence::EmergenceEntry;
use hydra_swarm::health::SwarmHealthLevel;
use hydra_swarm::swarm::Swarm;
use hydra_trust::TrustTier;
use uuid::Uuid;

static mut PASS: u32 = 0;
static mut FAIL: u32 = 0;

macro_rules! assert_harness {
    ($cond:expr, $name:expr) => {
        if $cond {
            eprintln!("  PASS: {}", $name);
            unsafe {
                PASS += 1;
            }
        } else {
            eprintln!("  FAIL: {}", $name);
            unsafe {
                FAIL += 1;
            }
        }
    };
}

fn spawn_agent(reg: &mut FleetRegistry, name: &str, spec: AgentSpecialization) -> Uuid {
    reg.spawn(name, spec, "harness-root", 0.5, TrustTier::Silver)
        .expect("spawn should succeed")
}

fn run_fleet_tests() {
    eprintln!("\n=== Fleet Tests ===");

    // 1. Standard spawn
    let mut reg = FleetRegistry::new();
    let id = spawn_agent(&mut reg, "agent-1", AgentSpecialization::Generalist);
    assert_harness!(reg.agent_count() == 1, "standard spawn");
    assert_harness!(reg.get_agent(&id).is_some(), "agent exists after spawn");

    // 2. Task lifecycle
    let task = FleetTask::new(TaskType::CodeAnalysis, "analyse", 5, "root");
    let task_id = task.id;
    let assigned = reg.assign_task(task, &AssignmentStrategy::FirstAvailable);
    assert_harness!(assigned.is_ok(), "task assignment succeeds");

    let result = AgentResult::new(id, task_id, ResultOutcome::Success, "done");
    assert_harness!(result.is_ok(), "result creation succeeds");

    let receipt = reg.submit_result(result.unwrap());
    assert_harness!(receipt.is_ok(), "result submission succeeds");
    assert_harness!(!reg.receipts().is_empty(), "receipt recorded");

    // 3. Receipt before consumption
    let receipt_val = receipt.unwrap();
    assert_harness!(
        receipt_val.result_id != Uuid::nil(),
        "receipt has valid result_id"
    );
    assert_harness!(
        receipt_val.agent_id == id,
        "receipt references correct agent"
    );

    // 4. Quarantine blocks tasks
    let mut reg2 = FleetRegistry::new();
    let qid = spawn_agent(&mut reg2, "q-agent", AgentSpecialization::Generalist);
    reg2.quarantine(&qid).expect("quarantine ok");
    let task2 = FleetTask::new(TaskType::Testing, "test", 3, "root");
    let assign2 = reg2.assign_task(task2, &AssignmentStrategy::FirstAvailable);
    assert_harness!(assign2.is_err(), "quarantined agent blocks task assignment");

    // 5. Capability match
    let mut reg3 = FleetRegistry::new();
    spawn_agent(&mut reg3, "analyst", AgentSpecialization::Analyst);
    spawn_agent(&mut reg3, "tester", AgentSpecialization::Tester);
    let task3 = FleetTask::new(TaskType::CodeAnalysis, "analyse code", 5, "root");
    let matched = reg3.assign_task(task3, &AssignmentStrategy::CapabilityMatch);
    assert_harness!(matched.is_ok(), "capability match assignment succeeds");
    let matched_id = matched.unwrap();
    let matched_agent = reg3.get_agent(&matched_id).unwrap();
    assert_harness!(
        matched_agent.specialization == AgentSpecialization::Analyst,
        "capability match selects analyst"
    );

    // 6. Multi-agent fleet
    let mut reg4 = FleetRegistry::new();
    for i in 0..5 {
        spawn_agent(
            &mut reg4,
            &format!("agent-{i}"),
            AgentSpecialization::Generalist,
        );
    }
    assert_harness!(reg4.agent_count() == 5, "multi-agent fleet has 5 agents");

    // 7. Priority clamped
    let task4 = FleetTask::new(TaskType::Debugging, "debug", 255, "root");
    assert_harness!(task4.priority == 10, "priority clamped at 10");

    // 8. Low trust rejected
    let mut reg5 = FleetRegistry::new();
    let low = reg5.spawn(
        "low",
        AgentSpecialization::Generalist,
        "root",
        0.1,
        TrustTier::Bronze,
    );
    assert_harness!(low.is_err(), "low trust spawn rejected");

    // 9. Agent success rate
    let mut reg6 = FleetRegistry::new();
    let sid = spawn_agent(&mut reg6, "rate-agent", AgentSpecialization::Generalist);
    let agent = reg6.get_agent(&sid).unwrap();
    assert_harness!(
        (agent.success_rate() - 1.0).abs() < f64::EPSILON,
        "new agent has 1.0 success rate"
    );
}

fn run_swarm_tests() {
    eprintln!("\n=== Swarm Tests ===");

    // 10. Consensus detection — 3 agents agree
    let answers = vec![
        AgentAnswer::from_text(Uuid::new_v4(), "the answer is forty two"),
        AgentAnswer::from_text(Uuid::new_v4(), "the answer is forty two"),
        AgentAnswer::from_text(Uuid::new_v4(), "the answer is forty two"),
    ];
    let reg = FleetRegistry::new();
    let swarm = Swarm::new(reg);
    let signal = swarm.evaluate_consensus(&answers);
    assert_harness!(signal.is_ok(), "consensus evaluation succeeds");
    let sig = signal.unwrap();
    assert_harness!(sig.reached, "consensus reached with 3 identical answers");
    assert_harness!(sig.is_strong(), "consensus is strong");

    // 11. No false consensus on disagreement
    let disagreement = vec![
        AgentAnswer::from_text(Uuid::new_v4(), "alpha beta gamma"),
        AgentAnswer::from_text(Uuid::new_v4(), "delta epsilon zeta"),
        AgentAnswer::from_text(Uuid::new_v4(), "eta theta iota"),
    ];
    let reg2 = FleetRegistry::new();
    let swarm2 = Swarm::new(reg2);
    let sig2 = swarm2.evaluate_consensus(&disagreement).unwrap();
    assert_harness!(!sig2.reached, "no false consensus on disagreement");

    // 12. Emergence recording
    let reg3 = FleetRegistry::new();
    let mut swarm3 = Swarm::new(reg3);
    let entry = EmergenceEntry::new("collective pattern detected", 4, 0.9);
    let appended = swarm3.record_emergence(entry);
    assert_harness!(appended, "emergence entry appended");
    assert_harness!(swarm3.emergence_count() == 1, "emergence count is 1");

    // 13. Emergence count only grows
    let entry2 = EmergenceEntry::new("second pattern", 3, 0.8);
    swarm3.record_emergence(entry2);
    assert_harness!(swarm3.emergence_count() == 2, "emergence count grows to 2");

    // 14. Lyapunov delta positive when healthy
    let mut reg4 = FleetRegistry::new();
    spawn_agent_on(&mut reg4, "h1");
    spawn_agent_on(&mut reg4, "h2");
    let mut swarm4 = Swarm::new(reg4);
    let delta = swarm4.lyapunov_delta();
    assert_harness!(delta > 0.0, "lyapunov delta positive when healthy");

    // 15. Health level is Healthy with all active
    let health = swarm4.health();
    assert_harness!(
        health.level == SwarmHealthLevel::Healthy,
        "health level is Healthy"
    );
}

fn run_integration_tests() {
    eprintln!("\n=== Integration Tests ===");

    // 16-22: 4-agent fleet → complete same task → consensus → health
    let mut reg = FleetRegistry::new();
    let mut agent_ids = Vec::new();
    for i in 0..4 {
        let id = spawn_agent(
            &mut reg,
            &format!("int-agent-{i}"),
            AgentSpecialization::Analyst,
        );
        agent_ids.push(id);
    }
    assert_harness!(reg.agent_count() == 4, "integration: 4 agents spawned");

    // Assign and complete tasks
    let mut receipts: Vec<ResultReceipt> = Vec::new();
    let mut answers = Vec::new();
    for &aid in &agent_ids {
        let task = FleetTask::new(
            TaskType::CodeAnalysis,
            "analyse integration module",
            7,
            "int-root",
        );
        let task_id = task.id;
        reg.assign_task(task, &AssignmentStrategy::FirstAvailable)
            .expect("assign ok");

        let result = AgentResult::new(
            aid,
            task_id,
            ResultOutcome::Success,
            "module is well structured",
        )
        .expect("result ok");
        let receipt = reg.submit_result(result).expect("submit ok");
        receipts.push(receipt);

        answers.push(AgentAnswer::from_text(
            aid,
            "module is well structured and correct",
        ));
    }

    assert_harness!(receipts.len() == 4, "integration: all 4 results receipted");

    // Check consensus
    let mut swarm = Swarm::new(reg);
    let signal = swarm.evaluate_consensus(&answers).unwrap();
    assert_harness!(
        signal.reached,
        "integration: consensus reached among 4 agents"
    );

    // Check health
    let health = swarm.health();
    assert_harness!(
        health.level == SwarmHealthLevel::Healthy,
        "integration: swarm is healthy"
    );

    // Positive Lyapunov
    let delta = swarm.lyapunov_delta();
    assert_harness!(delta > 0.0, "integration: positive lyapunov delta");

    // All receipted
    for r in &receipts {
        assert_harness!(
            r.outcome == ResultOutcome::Success,
            &format!("integration: receipt {} is Success", r.receipt_id)
        );
    }

    // Record emergence from this integration
    let entry = EmergenceEntry::new("4-agent unanimous analysis", 4, signal.similarity);
    let recorded = swarm.record_emergence(entry);
    assert_harness!(recorded, "integration: emergence recorded");
}

fn spawn_agent_on(reg: &mut FleetRegistry, name: &str) -> Uuid {
    spawn_agent(reg, name, AgentSpecialization::Generalist)
}

fn main() {
    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║  Hydra Phase 9 — Combined Test Harness   ║");
    eprintln!("╚══════════════════════════════════════════╝");

    run_fleet_tests();
    run_swarm_tests();
    run_integration_tests();

    let (pass, fail) = unsafe { (PASS, FAIL) };
    eprintln!("\n══════════════════════════════════════");
    eprintln!(
        "Results: {pass} passed, {fail} failed, {} total",
        pass + fail
    );
    if fail > 0 {
        eprintln!("SOME TESTS FAILED");
        std::process::exit(1);
    } else {
        eprintln!("ALL TESTS PASSED");
    }
}
