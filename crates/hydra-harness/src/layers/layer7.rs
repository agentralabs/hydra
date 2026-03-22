//! Tests all Layer 7 capabilities.

use crate::TestResult;
use std::time::Instant;

pub fn run() -> Vec<TestResult> {
    let mut results = Vec::new();

    results.extend(test_succession());
    results.extend(test_legacy());
    results.extend(test_influence());
    results.extend(test_continuity());

    let passed = results.iter().filter(|r| r.passed).count();
    println!("  Layer 7: {}/{} passed", passed, results.len());
    results
}

fn test_succession() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_succession::{SuccessionEngine, InstanceState};
        let mut engine = SuccessionEngine::new();
        let state = InstanceState {
            instance_id:          "harness-v1".into(),
            lineage_id:           "hydra-agentra-lineage".into(),
            days_running:         365,
            soul_entries:         36,
            genome_entries:       730,
            calibration_profiles: 5,
        };
        let result = engine.full_succession(&state)
            .expect("full_succession must succeed");
        assert_eq!(result.wisdom_days, 365);
        assert!(result.soul_entries > 0);
        assert!(result.genome_entries > 0);
        assert!(engine.has_imported());
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-succession: entity transfer");
            results.push(TestResult::pass("hydra-succession", "entity_transfer",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-succession: {}", err);
            results.push(TestResult::fail("hydra-succession", "entity_transfer",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_legacy() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_legacy::LegacyEngine;
        use hydra_succession::{InstanceState, SuccessionExporter};
        let pkg = SuccessionExporter::new().export(&InstanceState {
            instance_id: "harness".into(),
            lineage_id: "hydra-agentra-lineage".into(),
            days_running: 400,
            soul_entries: 40,
            genome_entries: 800,
            calibration_profiles: 5,
        }).expect("export must succeed");
        let mut engine = LegacyEngine::new();
        let artifact = engine.publish_knowledge(&pkg, "engineering")
            .expect("publish_knowledge must succeed");
        assert!(artifact.verify_integrity());
        assert!(artifact.source_days == 400);
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-legacy: knowledge export");
            results.push(TestResult::pass("hydra-legacy", "knowledge_export",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-legacy: {}", err);
            results.push(TestResult::fail("hydra-legacy", "knowledge_export",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_influence() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_influence::{InfluenceEngine, PatternCategory, DiscoveryQuery};
        let mut engine = InfluenceEngine::new();
        let id = engine.publish(
            "hydra-agentra-lineage",
            "Circuit Breaker Standard",
            "Prevent cascade failures",
            PatternCategory::Engineering,
            vec!["engineering".into()],
            "Install at every dependency boundary",
            10, 0.88, 365,
        ).expect("publish must succeed");
        engine.adopt(&id, "harness-peer").expect("adopt must succeed");
        engine.record_outcome(&id, "harness-peer", true);
        let results_d = engine.discover(&DiscoveryQuery::default());
        assert!(!results_d.is_empty(),
            "Published pattern must be discoverable");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-influence: publish + adopt + feedback");
            results.push(TestResult::pass("hydra-influence",
                "publish_adopt_feedback", start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-influence: {}", err);
            results.push(TestResult::fail("hydra-influence",
                "publish_adopt_feedback", &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_continuity() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_continuity::ContinuityEngine;
        use hydra_succession::{InstanceState, SuccessionExporter};
        let pkg = SuccessionExporter::new().export(&InstanceState {
            instance_id: "harness".into(),
            lineage_id: "hydra-agentra-lineage".into(),
            days_running: 730,
            soul_entries: 73,
            genome_entries: 1460,
            calibration_profiles: 5,
        }).expect("export must succeed");
        let mut engine = ContinuityEngine::new();
        engine.record_from_succession(&pkg);
        assert!(engine.total_checkpoint_count() > 0);
        let proven = engine.prove_lineage("hydra-agentra-lineage")
            .expect("prove_lineage must succeed");
        assert!(proven, "Lineage must be provable from arc");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-continuity: identity arc + lineage proof");
            results.push(TestResult::pass("hydra-continuity", "identity_arc",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-continuity: {}", err);
            results.push(TestResult::fail("hydra-continuity", "identity_arc",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}
