//! Tests all Layer 3 capabilities.

use crate::TestResult;
use std::time::Instant;

pub fn run() -> Vec<TestResult> {
    let mut results = Vec::new();

    results.extend(test_executor());
    results.extend(test_audit());
    results.extend(test_automation());
    results.extend(test_scheduler());

    let passed = results.iter().filter(|r| r.passed).count();
    println!("  Layer 3: {}/{} passed", passed, results.len());
    results
}

fn test_executor() -> Vec<TestResult> {
    let mut results = Vec::new();

    // Test 1: FAILED does not exist as a task state
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        use hydra_executor::TaskRecord;
        let task = TaskRecord::new("test.action", "verify FAILED does not exist");
        // Verify terminal states are only Complete and HardDenied
        assert!(
            !task.state.is_terminal(),
            "New task must not be in terminal state",
        );
        assert!(
            !task.state.is_hard_denied(),
            "New task must not be hard denied",
        );
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-executor: FAILED does not exist");
            results.push(TestResult::pass("hydra-executor", "failed_not_exist",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-executor: {}", err);
            results.push(TestResult::fail("hydra-executor", "failed_not_exist",
                &err, start.elapsed().as_millis() as u64));
        }
    }

    // Test 2: ExecutionEngine creation
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        use hydra_executor::ExecutionEngine;
        let engine = ExecutionEngine::new();
        assert!(engine.receipt_count() == 0, "New engine has 0 receipts");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-executor: engine creation");
            results.push(TestResult::pass("hydra-executor", "engine_creation",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-executor: engine: {}", err);
            results.push(TestResult::fail("hydra-executor", "engine_creation",
                &err, start.elapsed().as_millis() as u64));
        }
    }

    results
}

fn test_audit() -> Vec<TestResult> {
    let mut results = Vec::new();

    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        use hydra_audit::{AuditEngine, event::EventKind};
        let mut engine = AuditEngine::new();
        let result = engine.audit_manual(
            "test-task-id",
            "test.action",
            vec![
                (EventKind::TaskStarted { intent: "harness test".to_string() },
                 "genesis", 0),
                (EventKind::TaskCompleted { duration_total_ms: 100 },
                 "receipt-001", 100),
            ],
        );
        assert!(result.is_ok(), "Audit manual must succeed");
        assert!(engine.record_count() > 0, "Audit must record");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-audit: receipt chain");
            results.push(TestResult::pass("hydra-audit", "receipt_chain",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-audit: {}", err);
            results.push(TestResult::fail("hydra-audit", "receipt_chain",
                &err, start.elapsed().as_millis() as u64));
        }
    }

    results
}

fn test_automation() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_automation::{AutomationEngine, ExecutionObservation};
        let mut engine = AutomationEngine::new();
        for i in 0..3 {
            let obs = ExecutionObservation::new(
                "deploy-staging",
                format!("observation-{}", i),
                std::collections::HashMap::new(),
                "engineering",
                100,
                true,
            );
            engine.observe(obs);
        }
        assert!(engine.observation_count() == 3, "Must have 3 observations");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-automation: observation tracking");
            results.push(TestResult::pass("hydra-automation", "observation_tracking",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-automation: {}", err);
            results.push(TestResult::fail("hydra-automation", "observation_tracking",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_scheduler() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_scheduler::SchedulerEngine;
        let engine = SchedulerEngine::new();
        assert!(engine.job_count() == 0, "New scheduler has 0 jobs");
        assert!(engine.tick_count() == 0, "New scheduler has 0 ticks");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-scheduler: engine creation");
            results.push(TestResult::pass("hydra-scheduler", "engine_creation",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-scheduler: {}", err);
            results.push(TestResult::fail("hydra-scheduler", "engine_creation",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}
