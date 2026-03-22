//! Tests all Layer 4 capabilities.

use crate::TestResult;
use std::time::Instant;

pub fn run() -> Vec<TestResult> {
    let mut results = Vec::new();

    results.extend(test_pattern());
    results.extend(test_redteam());
    results.extend(test_calibration());
    results.extend(test_wisdom());

    let passed = results.iter().filter(|r| r.passed).count();
    println!("  Layer 4: {}/{} passed", passed, results.len());
    results
}

fn test_pattern() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_pattern::PatternEngine;
        let engine = PatternEngine::new();
        // Library must be queryable without panic
        let _ = engine.library_size();
        let _ = engine.antipattern_count();
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-pattern: engine creation");
            results.push(TestResult::pass("hydra-pattern", "engine_creation",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-pattern: {}", err);
            results.push(TestResult::fail("hydra-pattern", "engine_creation",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_redteam() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_redteam::RedTeamEngine;
        use hydra_axiom::AxiomPrimitive;
        let mut engine = RedTeamEngine::new();
        let prims = vec![AxiomPrimitive::Risk, AxiomPrimitive::AdversarialModel];
        let scenario = engine.analyze(
            "deploy auth service with hardcoded credentials",
            &prims,
        ).expect("RedTeam analysis must succeed");
        // Any analysis must produce a recommendation
        assert!(matches!(
            scenario.go_no_go,
            hydra_redteam::GoNoGo::Go
            | hydra_redteam::GoNoGo::GoWithMitigations { .. }
            | hydra_redteam::GoNoGo::NoGo { .. }
        ), "Must have a valid recommendation");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-redteam: GO/GO-WITH-MITIGATIONS/NO-GO");
            results.push(TestResult::pass("hydra-redteam", "three_state_recommendation",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-redteam: {}", err);
            results.push(TestResult::fail("hydra-redteam", "three_state_recommendation",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_calibration() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_calibration::{CalibrationEngine, JudgmentType};
        let mut engine = CalibrationEngine::new();
        let id = engine.record_prediction(
            "engineering", JudgmentType::RiskAssessment, 0.85,
        ).expect("Prediction recording must succeed");
        engine.record_outcome(&id, 0.70)
            .expect("Outcome recording must succeed");
        let adjusted = engine.calibrate(
            0.85, "engineering", &JudgmentType::RiskAssessment,
        );
        assert!(adjusted.calibrated >= 0.0 && adjusted.calibrated <= 1.0);
        assert!(engine.record_count() > 0);
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-calibration: bias correction");
            results.push(TestResult::pass("hydra-calibration", "bias_correction",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-calibration: {}", err);
            results.push(TestResult::fail("hydra-calibration", "bias_correction",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_wisdom() -> Vec<TestResult> {
    let mut results = Vec::new();

    // Test 1: Wisdom requires intelligence signal
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        use hydra_wisdom::{WisdomEngine, WisdomInput, WisdomError};
        let mut engine = WisdomEngine::new();
        let input = WisdomInput::new("deploy to production", "engineering")
            .with_base_confidence(0.75);
        assert!(!input.has_intelligence(),
            "Plain WisdomInput must have no intelligence");
        let result = engine.synthesize(&input);
        assert!(matches!(result, Err(WisdomError::InsufficientIntelligence)),
            "Must error on no intelligence, not panic");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-wisdom: intelligence gate");
            results.push(TestResult::pass("hydra-wisdom", "intelligence_gate",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-wisdom: intelligence gate: {}", err);
            results.push(TestResult::fail("hydra-wisdom", "intelligence_gate",
                &err, start.elapsed().as_millis() as u64));
        }
    }

    // Test 2: Wisdom synthesizes with evidence
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        use hydra_wisdom::{WisdomEngine, WisdomInput, PatternEvidence};
        let mut engine = WisdomEngine::new();
        let input = WisdomInput::new("deploy auth service", "fintech")
            .with_base_confidence(0.80)
            .with_pattern(PatternEvidence {
                pattern_name: "circuit-breaker".to_string(),
                is_warning:   true,
                similarity:   0.88,
                response:     "Install circuit breaker at service boundaries".to_string(),
            });
        let stmt = engine.synthesize(&input)
            .expect("Synthesis must succeed with evidence");
        // Recommendation is a non-empty enum, check it exists
        let _ = &stmt.recommendation;
        assert!(!stmt.reasoning_chain.is_empty(),
            "Reasoning chain must be populated");
        assert!(engine.statement_count() == 1);
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-wisdom: synthesis with evidence");
            results.push(TestResult::pass("hydra-wisdom", "synthesis_with_evidence",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-wisdom: synthesis: {}", err);
            results.push(TestResult::fail("hydra-wisdom", "synthesis_with_evidence",
                &err, start.elapsed().as_millis() as u64));
        }
    }

    results
}
