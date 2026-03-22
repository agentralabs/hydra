//! Test harness for hydra-kernel.
//!
//! This is a standalone binary that exercises the kernel's core functionality
//! in sequence. It is NOT a test suite — it is a demonstration that the kernel
//! can boot, tick, process commands, and shut down.

use hydra_constitution::declarations::HardStop;
use hydra_kernel::{
    boot, constants, equation, health, intent, invariants, loop_active, loop_ambient, loop_dream,
    state::{HydraState, KernelPhase},
    task_engine::{ManagedTask, TaskEngine},
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("hydra_kernel=info")
        .init();

    println!(
        "=== hydra-kernel test harness v{} ===",
        constants::KERNEL_VERSION
    );
    println!();

    let mut passed = 0u32;
    let mut failed = 0u32;

    // Scenario 1: Boot sequence
    print_scenario("Boot sequence");
    match boot::run_boot_sequence().await {
        Ok(result) => {
            println!("  PASS: Boot completed in {}ms", result.boot_duration_ms);
            println!("  Phases: {:?}", result.phases_completed);
            passed += 1;
        }
        Err(e) => {
            println!("  FAIL: {e}");
            failed += 1;
        }
    }

    // Scenario 2: Initial state
    print_scenario("Initial state");
    let state = HydraState::initial();
    if state.is_stable() && state.step_count == 0 {
        println!("  PASS: Initial state is stable, step=0");
        passed += 1;
    } else {
        println!("  FAIL: Initial state unexpected");
        failed += 1;
    }

    // Scenario 3: Equation step
    print_scenario("Equation step");
    let step = equation::EquationStep::compute(&state);
    println!("  {}", step.summary());
    if step.dpsi_dt.is_finite() {
        println!("  PASS: Equation step produces finite result");
        passed += 1;
    } else {
        println!("  FAIL: Non-finite equation result");
        failed += 1;
    }

    // Scenario 4: Euler integration
    print_scenario("Euler integration");
    let next = equation::integrate_euler(&state, 0.1);
    if next.step_count == 1 && next.lyapunov_value.is_finite() {
        println!(
            "  PASS: Integration step=1, V(Psi)={:.4}",
            next.lyapunov_value
        );
        passed += 1;
    } else {
        println!("  FAIL: Integration unexpected");
        failed += 1;
    }

    // Scenario 5: Invariant check
    print_scenario("Invariant check on initial state");
    let results = invariants::check_all(&state);
    if results.all_passed {
        println!("  PASS: All {} invariants pass", results.results.len());
        passed += 1;
    } else {
        println!("  FAIL: Invariant failed: {:?}", results.first_failure());
        failed += 1;
    }

    // Scenario 6: Invariant failure detection
    print_scenario("Invariant failure detection");
    let mut bad_state = state.clone();
    bad_state.lyapunov_value = -1.0;
    let bad_results = invariants::check_all(&bad_state);
    if !bad_results.all_passed {
        println!("  PASS: Correctly detected invariant failure");
        passed += 1;
    } else {
        println!("  FAIL: Should have detected failure");
        failed += 1;
    }

    // Scenario 7: Task engine
    print_scenario("Task engine");
    let mut engine = TaskEngine::new();
    let task = ManagedTask::new("build agentic-data v0.2.0");
    match engine.submit(task) {
        Ok(id) => {
            println!("  PASS: Task submitted: {id}");
            passed += 1;
        }
        Err(e) => {
            println!("  FAIL: {e}");
            failed += 1;
        }
    }

    // Scenario 8: Task obstacle navigation
    print_scenario("Task obstacle navigation");
    let mut task = ManagedTask::new("deploy to production");
    task.hit_obstacle(hydra_constitution::task::ObstacleType::Timeout { duration_ms: 5000 });
    task.reroute(hydra_constitution::task::ObstacleType::Timeout { duration_ms: 5000 });
    task.resume_active();
    task.complete();
    if task.state.is_terminal() && task.attempts.len() == 2 {
        println!("  PASS: Task navigated obstacle and completed");
        passed += 1;
    } else {
        println!("  FAIL: Task lifecycle unexpected");
        failed += 1;
    }

    // Scenario 9: Active loop - execute
    print_scenario("Active loop - execute command");
    let mut engine2 = TaskEngine::new();
    let cmd = loop_active::ActiveCommand::Execute {
        description: "test task".to_string(),
    };
    match loop_active::process_command(&cmd, &state, &mut engine2).await {
        Ok(result) => {
            if result.accepted {
                println!("  PASS: Command accepted, task={:?}", result.task_id);
                passed += 1;
            } else {
                println!("  FAIL: Command rejected: {}", result.message);
                failed += 1;
            }
        }
        Err(e) => {
            println!("  FAIL: {e}");
            failed += 1;
        }
    }

    // Scenario 10: Active loop - query state
    print_scenario("Active loop - query state");
    let cmd = loop_active::ActiveCommand::QueryState;
    match loop_active::process_command(&cmd, &state, &mut engine2).await {
        Ok(result) => {
            println!("  PASS: {}", result.message);
            passed += 1;
        }
        Err(e) => {
            println!("  FAIL: {e}");
            failed += 1;
        }
    }

    // Scenario 11: Ambient tick
    print_scenario("Ambient tick");
    let tick_result = loop_ambient::tick(&state, 0.1);
    println!("  {}", tick_result.summary);
    if tick_result.invariants_ok {
        println!("  PASS: Ambient tick healthy");
        passed += 1;
    } else {
        println!("  FAIL: Ambient tick unhealthy");
        failed += 1;
    }

    // Scenario 12: Multiple ambient ticks
    print_scenario("10 ambient ticks");
    let mut running_state = state.clone();
    for _ in 0..10 {
        let result = loop_ambient::tick(&running_state, 0.1);
        running_state = result.state;
    }
    if running_state.step_count == 10 {
        println!(
            "  PASS: 10 ticks, V(Psi)={:.4}",
            running_state.lyapunov_value
        );
        passed += 1;
    } else {
        println!("  FAIL: Expected 10 steps");
        failed += 1;
    }

    // Scenario 13: Dream cycle
    print_scenario("Dream cycle");
    let dream = loop_dream::cycle(&state);
    println!("  {}", dream.summary);
    println!("  PASS: Dream cycle completed");
    passed += 1;

    // Scenario 14: Dream cycle with beliefs
    print_scenario("Dream cycle with beliefs");
    let mut belief_state = state.clone();
    belief_state.growth_state.beliefs_revised = 7;
    let dream = loop_dream::cycle(&belief_state);
    if dream.did_work && dream.beliefs_consolidated == 7 {
        println!(
            "  PASS: Consolidated {} beliefs",
            dream.beliefs_consolidated
        );
        passed += 1;
    } else {
        println!("  FAIL: Dream work unexpected");
        failed += 1;
    }

    // Scenario 15: Health report
    print_scenario("Health report");
    let engine3 = TaskEngine::new();
    let health_report = health::KernelHealth::capture(&state, &KernelPhase::Alive, &engine3);
    println!("  {}", health_report.status_line());
    if health_report.invariants_ok {
        println!("  PASS: Health report clean");
        passed += 1;
    } else {
        println!("  FAIL: Health report shows issues");
        failed += 1;
    }

    // Scenario 16: Intent parsing
    print_scenario("Intent parsing");
    match intent::parse_intent("build the project", 2) {
        Ok((cmd, resolved)) => {
            println!("  Command: {:?}", std::mem::discriminant(&cmd));
            println!(
                "  Signal chain complete: {}",
                resolved.signal.chain_is_complete()
            );
            println!("  PASS: Intent parsed");
            passed += 1;
        }
        Err(e) => {
            println!("  FAIL: {e}");
            failed += 1;
        }
    }

    // Scenario 17: Intent parsing - shutdown
    print_scenario("Intent parsing - shutdown");
    match intent::parse_intent("shutdown", 2) {
        Ok((cmd, _)) => {
            if matches!(cmd, loop_active::ActiveCommand::Shutdown) {
                println!("  PASS: Shutdown intent recognized");
                passed += 1;
            } else {
                println!("  FAIL: Wrong command type");
                failed += 1;
            }
        }
        Err(e) => {
            println!("  FAIL: {e}");
            failed += 1;
        }
    }

    // Scenario 18: Task hard deny
    print_scenario("Task hard deny");
    let mut deny_task = ManagedTask::new("do something forbidden");
    deny_task.hard_deny(HardStop::ConstitutionalViolationRequired {
        law: hydra_constitution::laws::LawId::Law3MemorySovereignty,
        reason: "constitutional violation".to_string(),
    });
    if deny_task.state.is_terminal() {
        println!("  PASS: Task hard denied correctly");
        passed += 1;
    } else {
        println!("  FAIL: Task not terminal after hard deny");
        failed += 1;
    }

    // Scenario 19: Stability under adversarial conditions
    print_scenario("Adversarial conditions");
    let mut adversarial = state.clone();
    adversarial.trust_field.adversarial_detected = true;
    adversarial.trust_field.average_trust = 0.05;
    let adv_results = invariants::check_all(&adversarial);
    if !adv_results.all_passed {
        println!("  PASS: Adversarial conditions detected by invariants");
        passed += 1;
    } else {
        println!("  FAIL: Adversarial conditions not detected");
        failed += 1;
    }

    // Scenario 20: Full lifecycle
    print_scenario("Full lifecycle: boot -> tick -> command -> tick -> shutdown");
    let boot_result = boot::run_boot_sequence().await;
    if let Ok(boot) = boot_result {
        let mut lifecycle_state = boot.state;
        let mut lifecycle_engine = TaskEngine::new();

        // Tick a few times
        for _ in 0..5 {
            let r = loop_ambient::tick(&lifecycle_state, 0.1);
            lifecycle_state = r.state;
        }

        // Process a command
        let cmd = loop_active::ActiveCommand::Execute {
            description: "lifecycle test".to_string(),
        };
        let _ = loop_active::process_command(&cmd, &lifecycle_state, &mut lifecycle_engine).await;

        // Tick more
        for _ in 0..5 {
            let r = loop_ambient::tick(&lifecycle_state, 0.1);
            lifecycle_state = r.state;
        }

        if lifecycle_state.step_count == 10 && lifecycle_engine.active_count() == 1 {
            println!("  PASS: Full lifecycle completed");
            passed += 1;
        } else {
            println!(
                "  FAIL: step={}, tasks={}",
                lifecycle_state.step_count,
                lifecycle_engine.active_count()
            );
            failed += 1;
        }
    } else {
        println!("  FAIL: Boot failed");
        failed += 1;
    }

    // Summary
    println!();
    println!("=== Results: {passed} passed, {failed} failed ===");
    if failed > 0 {
        std::process::exit(1);
    }
}

fn print_scenario(name: &str) {
    println!("--- Scenario: {name} ---");
}
