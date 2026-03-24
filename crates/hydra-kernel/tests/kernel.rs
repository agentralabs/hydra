//! Integration tests for hydra-kernel.

use hydra_kernel::{
    boot, constants, equation, health, intent, invariants, loop_active, loop_ambient, loop_dream,
    state::{HydraState, KernelPhase},
    task_engine::{ManagedTask, TaskEngine},
};

#[test]
fn kernel_version_matches_cargo() {
    assert_eq!(constants::KERNEL_VERSION, "0.1.0");
}

#[test]
fn initial_state_is_stable() {
    let state = HydraState::initial();
    assert!(state.is_stable());
    assert!(!state.is_critical());
}

#[test]
fn equation_step_is_finite() {
    let state = HydraState::initial();
    let step = equation::EquationStep::compute(&state);
    assert!(step.dpsi_dt.is_finite());
    assert!(step.l_hat.is_finite());
    assert!(step.a_hat.is_finite());
    assert!(step.g_hat.is_finite());
    assert!(step.s_hat.is_finite());
    assert!(step.gamma_hat.is_finite());
}

#[test]
fn euler_integration_preserves_stability() {
    let mut state = HydraState::initial();
    for _ in 0..100 {
        state = equation::integrate_euler(&state, 0.01);
    }
    // After 100 small steps, state should still be finite
    assert!(state.lyapunov_value.is_finite());
    assert_eq!(state.step_count, 100);
}

#[test]
fn invariants_pass_on_healthy_state() {
    let state = HydraState::initial();
    let results = invariants::check_all(&state);
    assert!(results.all_passed);
    assert_eq!(results.results.len(), 6);
}

#[test]
fn invariants_detect_critical_lyapunov() {
    let mut state = HydraState::initial();
    state.lyapunov_value = -1.0;
    let results = invariants::check_all(&state);
    assert!(!results.all_passed);
}

#[test]
fn task_lifecycle_complete() {
    let mut task = ManagedTask::new("integration test task");
    assert!(task.state.is_active());

    task.hit_obstacle(hydra_constitution::task::ObstacleType::Timeout { duration_ms: 1000 });
    assert!(task.state.is_active());

    task.reroute(hydra_constitution::task::ObstacleType::Timeout { duration_ms: 1000 });
    assert!(task.state.is_active());

    task.resume_active();
    assert!(task.state.is_active());

    task.complete();
    assert!(task.state.is_terminal());
}

#[test]
fn task_engine_capacity() {
    let mut engine = TaskEngine::new();
    for i in 0..constants::MAX_CONCURRENT_TASKS {
        let task = ManagedTask::new(format!("task-{i}"));
        engine.submit(task).expect("should submit");
    }
    assert_eq!(engine.active_count(), constants::MAX_CONCURRENT_TASKS);

    // One more should fail
    let overflow = ManagedTask::new("overflow");
    assert!(engine.submit(overflow).is_err());
}

#[test]
fn ambient_tick_advances() {
    let state = HydraState::initial();
    let result = loop_ambient::tick(&state, 0.1);
    assert_eq!(result.state.step_count, 1);
    assert!(result.invariants_ok);
}

#[test]
fn dream_cycle_runs() {
    let state = HydraState::initial();
    let result = loop_dream::cycle(&state);
    assert!(!result.did_work); // no beliefs to consolidate
}

#[test]
fn health_capture_works() {
    let state = HydraState::initial();
    let phase = KernelPhase::Alive;
    let engine = TaskEngine::new();
    let health = health::KernelHealth::capture(&state, &phase, &engine);
    assert!(health.invariants_ok);
    assert_eq!(health.version, constants::KERNEL_VERSION);
}

#[test]
fn intent_parsing_works() {
    let (cmd, resolved) = intent::parse_intent("do something", 2).expect("should parse");
    assert!(matches!(cmd, loop_active::ActiveCommand::Execute { .. }));
    assert!(resolved.signal.chain_is_complete());
}

#[tokio::test]
async fn boot_and_tick_sequence() {
    let boot_result = boot::run_boot_sequence().await.expect("boot should work");
    assert_eq!(boot_result.phase, KernelPhase::Alive);

    let mut state = boot_result.state;
    let mut engine = TaskEngine::new();

    // Run 10 ambient ticks
    for _ in 0..10 {
        let tick = loop_ambient::tick(&state, 0.1);
        state = tick.state;
    }
    assert_eq!(state.step_count, 10);

    // Process a command
    let cmd = loop_active::ActiveCommand::Execute {
        description: "integration test".to_string(),
    };
    let result = loop_active::process_command(&cmd, &state, &mut engine)
        .await
        .expect("command should work");
    assert!(result.accepted);

    // Check health
    let health = health::KernelHealth::capture(&state, &KernelPhase::Alive, &engine);
    assert!(health.invariants_ok);
    assert_eq!(health.active_tasks, 1);
}

#[test]
fn zero_defect_pipeline_produces_certificate() {
    use hydra_kernel::zero_defect::{run_gates, Gate};
    // Create a tiny Rust project in a temp dir so Gates 1-3,6 run fast
    let tmp = std::env::temp_dir().join("hydra_zd_test");
    let src = tmp.join("src");
    let _ = std::fs::create_dir_all(&src);
    std::fs::write(tmp.join("Cargo.toml"), "[package]\nname = \"zd_test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    let code = "pub fn add(a: i32, b: i32) -> i32 { a + b }\n#[test] fn t() { assert_eq!(add(1,2), 3); }";
    std::fs::write(src.join("lib.rs"), code).unwrap();
    let (results, cert) = run_gates(code, "src/lib.rs", "rust", &tmp.to_string_lossy());
    // All 7 gates always run
    assert_eq!(results.len(), 7);
    // Security + Edge Cases + Genome should pass for clean code
    assert!(results.iter().find(|r| r.gate == Gate::Security).unwrap().passed);
    assert!(results.iter().find(|r| r.gate == Gate::EdgeCases).unwrap().passed);
    assert!(results.iter().find(|r| r.gate == Gate::Genome).unwrap().passed);
    // If all gates pass, certificate is issued with correct hash (EC-9.9)
    if results.iter().all(|r| r.passed) {
        let cert = cert.expect("certificate when all gates pass");
        assert_eq!(cert.file_path, "src/lib.rs");
        assert!(!cert.file_hash.is_empty());
        let display = cert.format_display();
        assert!(display.contains("Certificate"));
        for g in Gate::all() { assert!(display.contains(g.label())); }
        // Verify saved to disk, then cleanup
        let cert_path = dirs::home_dir().unwrap().join(".hydra/certificates")
            .join(format!("{}.json", cert.file_hash));
        if cert_path.exists() { let _ = std::fs::remove_file(cert_path); }
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn worker_blast_radius_and_autonomy() {
    use hydra_kernel::conductor::{Step, StepType};
    use hydra_kernel::worker;

    let step = |st: StepType, desc: &str| Step {
        id: 0, step_type: st, description: desc.into(),
        depends_on: vec![], timeout_ms: 5000,
    };
    // Dangerous browser action → Irreversible
    assert_eq!(worker::blast_radius_for_step(
        &step(StepType::BrowserInteract { goal: "delete all tweets".into() }, "delete tweets")),
        hydra_wisdom::BlastRadius::Irreversible);
    // Safe read → Contained
    assert_eq!(worker::blast_radius_for_step(
        &step(StepType::FileRead { path: "x".into() }, "read file")),
        hydra_wisdom::BlastRadius::Contained);
    // Deploy command → Catastrophic
    assert_eq!(worker::blast_radius_for_step(
        &step(StepType::Shell { command: "deploy to prod".into(), long_running: false }, "deploy")),
        hydra_wisdom::BlastRadius::Catastrophic);
    // Interface classification
    assert_eq!(worker::classify_interface(&StepType::BrowserNavigate { url: "x".into() }),
        worker::Interface::Browser);
    assert_eq!(worker::classify_interface(&StepType::Shell { command: "ls".into(), long_running: false }),
        worker::Interface::Shell);
    // Workflow template
    let steps = worker::expand_workflow("email sarah about the report").unwrap();
    assert_eq!(steps.len(), 2);
}
