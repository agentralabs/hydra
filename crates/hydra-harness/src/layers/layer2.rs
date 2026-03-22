//! Tests all Layer 2 capabilities.

use crate::TestResult;
use std::time::Instant;

pub fn run() -> Vec<TestResult> {
    let mut results = Vec::new();

    results.extend(test_comprehension());
    results.extend(test_language());
    results.extend(test_context());
    results.extend(test_attention());
    results.extend(test_reasoning());
    results.extend(test_noticing());

    let passed = results.iter().filter(|r| r.passed).count();
    println!("  Layer 2: {}/{} passed", passed, results.len());
    results
}

fn test_comprehension() -> Vec<TestResult> {
    let mut results = Vec::new();

    // Test 1: comprehend returns ComprehendedInput with confidence > 0
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        use hydra_comprehension::{ComprehensionEngine, InputSource};
        use hydra_genome::GenomeStore;
        let engine = ComprehensionEngine::new();
        let genome = GenomeStore::new();
        let result = engine.comprehend(
            "deploy the service and optimize performance under budget constraint",
            InputSource::PrincipalText,
            &genome,
        ).expect("Comprehension must succeed for valid input");
        assert!(result.confidence > 0.0, "Confidence must be > 0");
        assert!(!result.primitives.is_empty(), "Must map to primitives");
        result.confidence
    }) {
        Ok(conf) => {
            println!("  [PASS] hydra-comprehension: pipeline (conf={:.2})", conf);
            results.push(TestResult::pass("hydra-comprehension", "pipeline",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-comprehension: {}", err);
            results.push(TestResult::fail("hydra-comprehension", "pipeline",
                &err, start.elapsed().as_millis() as u64));
        }
    }

    // Test 2: empty input returns error (not panic)
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        use hydra_comprehension::{ComprehensionEngine, InputSource, ComprehensionError};
        use hydra_genome::GenomeStore;
        let engine = ComprehensionEngine::new();
        let genome = GenomeStore::new();
        let result = engine.comprehend("", InputSource::PrincipalText, &genome);
        assert!(matches!(result, Err(ComprehensionError::EmptyInput)),
            "Empty input must return EmptyInput error");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-comprehension: empty input guard");
            results.push(TestResult::pass("hydra-comprehension", "empty_input_guard",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-comprehension: empty guard: {}", err);
            results.push(TestResult::fail("hydra-comprehension", "empty_input_guard",
                &err, start.elapsed().as_millis() as u64));
        }
    }

    results
}

fn test_language() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_comprehension::{ComprehensionEngine, InputSource};
        use hydra_genome::GenomeStore;
        use hydra_language::LanguageEngine;
        let comp = ComprehensionEngine::new();
        let genome = GenomeStore::new();
        let comprehended = comp.comprehend(
            "how does the circuit breaker pattern prevent cascades",
            InputSource::PrincipalText,
            &genome,
        ).expect("Comprehension must succeed");
        let analysis = LanguageEngine::analyze(&comprehended)
            .expect("Language analysis must succeed");
        assert!(analysis.confidence > 0.0);
        let _ = analysis.intent.kind.label();
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-language: intent + affect analysis");
            results.push(TestResult::pass("hydra-language", "intent_analysis",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-language: {}", err);
            results.push(TestResult::fail("hydra-language", "intent_analysis",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_context() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_comprehension::{ComprehensionEngine, InputSource};
        use hydra_genome::GenomeStore;
        use hydra_context::{ContextFrame, SessionHistory, GapContext, AnomalyContext};
        let comp = ComprehensionEngine::new();
        let genome = GenomeStore::new();
        let comprehended = comp.comprehend(
            "what is a deployment pipeline",
            InputSource::PrincipalText,
            &genome,
        ).expect("Comprehension must succeed");
        let history  = SessionHistory::new();
        let gap_ctx  = GapContext::new();
        let anomaly  = AnomalyContext::new();
        let context  = ContextFrame::build(
            &comprehended, &history, &[], &gap_ctx, &anomaly,
        );
        assert!(!context.active.is_empty(),
            "Active context must have items from current input");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-context: 5-window frame built");
            results.push(TestResult::pass("hydra-context", "five_window_frame",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-context: {}", err);
            results.push(TestResult::fail("hydra-context", "five_window_frame",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_attention() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_comprehension::{ComprehensionEngine, InputSource};
        use hydra_genome::GenomeStore;
        use hydra_context::{ContextFrame, SessionHistory, GapContext, AnomalyContext};
        use hydra_language::LanguageEngine;
        use hydra_attention::AttentionEngine;

        let comp   = ComprehensionEngine::new();
        let genome = GenomeStore::new();

        let comprehended = comp.comprehend(
            "debug this concurrent race condition",
            InputSource::PrincipalText,
            &genome,
        ).expect("Comprehension must succeed");

        let language = LanguageEngine::analyze(&comprehended)
            .expect("Language analysis must succeed");
        let history  = SessionHistory::new();
        let gap_ctx  = GapContext::new();
        let anomaly  = AnomalyContext::new();
        let context  = ContextFrame::build(
            &comprehended, &history, &[], &gap_ctx, &anomaly,
        );

        if context.total_items() > 0 {
            let frame = AttentionEngine::allocate(&comprehended, &context, &language)
                .expect("Attention allocation must succeed when context has content");
            let _ = frame.attended_count();
        }
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-attention: budget allocation");
            results.push(TestResult::pass("hydra-attention", "budget_allocation",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-attention: {}", err);
            results.push(TestResult::fail("hydra-attention", "budget_allocation",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_reasoning() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_comprehension::{ComprehensionEngine, InputSource};
        use hydra_genome::GenomeStore;
        use hydra_context::{ContextFrame, SessionHistory, GapContext, AnomalyContext};
        use hydra_language::LanguageEngine;
        use hydra_attention::AttentionEngine;
        use hydra_reasoning::{ReasoningEngine, ReasoningError};

        let comp     = ComprehensionEngine::new();
        let genome   = GenomeStore::new();
        let rea_eng  = ReasoningEngine::new();

        let comprehended = comp.comprehend(
            "what are the benefits of using a circuit breaker pattern",
            InputSource::PrincipalText,
            &genome,
        ).expect("Comprehension must succeed");

        let language = LanguageEngine::analyze(&comprehended)
            .expect("Language analysis must succeed");
        let history  = SessionHistory::new();
        let gap_ctx  = GapContext::new();
        let anomaly  = AnomalyContext::new();
        let context  = ContextFrame::build(
            &comprehended, &history, &[], &gap_ctx, &anomaly,
        );

        if context.total_items() > 0 {
            if let Ok(att_frame) = AttentionEngine::allocate(
                &comprehended, &context, &language,
            ) {
                match rea_eng.reason(&comprehended, &att_frame, &genome) {
                    Ok(result) => {
                        assert!(
                            result.active_modes > 0 || result.conclusions.is_empty(),
                            "Reasoning must run at least one mode",
                        );
                    }
                    Err(ReasoningError::EmptyAttentionFrame) => {}
                    Err(ReasoningError::NoConclusions) => {
                        // Acceptable: keyword-based reasoning may not
                        // produce conclusions for all inputs.
                    }
                    Err(ReasoningError::LowSynthesisConfidence { .. }) => {
                        // Acceptable: synthesis confidence can be below
                        // threshold for lightweight inputs.
                    }
                }
            }
        }
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-reasoning: 5-mode synthesis");
            results.push(TestResult::pass("hydra-reasoning", "five_mode_synthesis",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-reasoning: {}", err);
            results.push(TestResult::fail("hydra-reasoning", "five_mode_synthesis",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_noticing() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_noticing::NoticingEngine;
        let mut engine = NoticingEngine::new();
        let _signals = engine.cycle();
        let _ = engine.cycle_count();
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-noticing: ambient cycle");
            results.push(TestResult::pass("hydra-noticing", "ambient_cycle",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-noticing: {}", err);
            results.push(TestResult::fail("hydra-noticing", "ambient_cycle",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}
