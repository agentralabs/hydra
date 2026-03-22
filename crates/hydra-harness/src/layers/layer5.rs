//! Tests all Layer 5 capabilities.

use crate::TestResult;
use std::time::Instant;

pub fn run() -> Vec<TestResult> {
    let mut results = Vec::new();

    results.extend(test_settlement());
    results.extend(test_attribution());
    results.extend(test_portfolio());
    results.extend(test_crystallizer());
    results.extend(test_exchange());

    let passed = results.iter().filter(|r| r.passed).count();
    println!("  Layer 5: {}/{} passed", passed, results.len());
    results
}

fn test_settlement() -> Vec<TestResult> {
    let mut results = Vec::new();

    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        use hydra_settlement::SettlementEngine;
        let mut engine = SettlementEngine::new();
        engine.settle_skill_action(
            "hydra-harness", "harness.test", "engineering",
            "automated test", 100, 500, true,
        ).expect("Settlement must succeed");
        assert!(engine.record_count() > 0);
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-settlement: cost accounting");
            results.push(TestResult::pass("hydra-settlement", "cost_accounting",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-settlement: {}", err);
            results.push(TestResult::fail("hydra-settlement", "cost_accounting",
                &err, start.elapsed().as_millis() as u64));
        }
    }

    results
}

fn test_attribution() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_attribution::AttributionEngine;
        let engine = AttributionEngine::new();
        assert!(engine.tree_count() == 0, "New engine has 0 trees");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-attribution: engine creation");
            results.push(TestResult::pass("hydra-attribution", "engine_creation",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-attribution: {}", err);
            results.push(TestResult::fail("hydra-attribution", "engine_creation",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_portfolio() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_portfolio::{PortfolioEngine, PortfolioObjective, ObjectiveCategory};
        let mut engine = PortfolioEngine::new();
        engine.add_objective(PortfolioObjective::new(
            "Improve test coverage", "Engineering quality",
            ObjectiveCategory::MaintenanceAndDebt,
            0.5, 0.8, 0.7, 0.6, 30.0,
        )).expect("add_objective must succeed");
        let alloc = engine.allocate(100.0, "harness-period")
            .expect("allocate must succeed");
        assert!(!alloc.allocations.is_empty(),
            "Allocation must produce results");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-portfolio: resource allocation");
            results.push(TestResult::pass("hydra-portfolio", "resource_allocation",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-portfolio: {}", err);
            results.push(TestResult::fail("hydra-portfolio", "resource_allocation",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_crystallizer() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_crystallizer::{CrystallizerEngine, CrystallizationSource};
        use hydra_settlement::{SettlementRecord, Outcome, CostItem, CostClass};
        let mut engine = CrystallizerEngine::new();
        let mut src = CrystallizationSource::new("engineering")
            .with_approach("use circuit breakers at service boundaries", 0.92)
            .with_approach("measure before optimizing", 0.91);
        for i in 0..5 {
            src = src.with_success(SettlementRecord::new(
                format!("s{}", i), "test.action", "engineering", "test",
                Outcome::Success { description: "ok".into() },
                vec![CostItem::new(CostClass::DirectExecution, 100, 5.0, 500)],
                500, 1,
            ));
        }
        let artifact = engine.crystallize_playbook(&src)
            .expect("Crystallization must succeed with 5 records");
        assert!(!artifact.content.is_empty(), "Artifact must have content");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-crystallizer: wisdom -> artifact");
            results.push(TestResult::pass("hydra-crystallizer", "wisdom_to_artifact",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-crystallizer: {}", err);
            results.push(TestResult::fail("hydra-crystallizer", "wisdom_to_artifact",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_exchange() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_exchange::{ExchangeEngine, ExchangeOffer, OfferKind};
        let mut engine = ExchangeEngine::new();
        engine.register_offer(ExchangeOffer::new(
            OfferKind::RedTeamAnalysis,
            "Pre-execution adversarial analysis",
            0.65, 8.0, None,
        ));
        assert!(engine.offer_count() > 0, "Must have registered offer");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-exchange: offer registration");
            results.push(TestResult::pass("hydra-exchange", "offer_registration",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-exchange: {}", err);
            results.push(TestResult::fail("hydra-exchange", "offer_registration",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}
