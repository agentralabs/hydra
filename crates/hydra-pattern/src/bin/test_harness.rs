//! Phase 29 test harness for hydra-pattern.
//! Run: cargo run -p hydra-pattern --bin test_harness

use hydra_axiom::primitives::AxiomPrimitive;
use hydra_pattern::PatternEngine;

struct Test {
    name: &'static str,
    passed: bool,
    notes: Option<String>,
}
impl Test {
    fn pass(name: &'static str) -> Self {
        Self {
            name,
            passed: true,
            notes: None,
        }
    }
    fn fail(name: &'static str, n: impl Into<String>) -> Self {
        Self {
            name,
            passed: false,
            notes: Some(n.into()),
        }
    }
}

fn main() {
    println!("=== hydra-pattern test harness ===\n");
    let mut tests = Vec::new();

    let engine = PatternEngine::new();

    // Test 1: library seeded
    if engine.library_size() >= 4 {
        tests.push(Test::pass("Library seeded with 4+ base patterns"));
    } else {
        tests.push(Test::fail(
            "Library seed count",
            format!("{}", engine.library_size()),
        ));
    }

    // Test 2: anti + success balance
    if engine.antipattern_count() >= 2 && engine.success_pattern_count() >= 2 {
        tests.push(Test::pass("2+ anti-patterns and 2+ success patterns"));
    } else {
        tests.push(Test::fail(
            "Kind balance",
            format!(
                "anti={} success={}",
                engine.antipattern_count(),
                engine.success_pattern_count()
            ),
        ));
    }

    // Test 3: cascade warning
    let prims = vec![
        AxiomPrimitive::Risk,
        AxiomPrimitive::CausalLink,
        AxiomPrimitive::Dependency,
    ];
    let warnings = engine.check_for_warnings(&prims);
    if !warnings.is_empty() && warnings.iter().any(|w| w.pattern_name.contains("Cascade")) {
        tests.push(Test::pass(
            "Cascade primitives trigger anti-pattern warning",
        ));
    } else {
        tests.push(Test::fail("Cascade warning", "no warning"));
    }

    // Test 4: trust escalation
    let trust_prims = vec![AxiomPrimitive::TrustRelation, AxiomPrimitive::Risk];
    let trust_warnings = engine.check_for_warnings(&trust_prims);
    if !trust_warnings.is_empty() {
        tests.push(Test::pass("Trust+risk triggers trust escalation warning"));
    } else {
        tests.push(Test::fail("Trust escalation", "no warning"));
    }

    // Test 5: match_primitives returns results
    let dep_prims = vec![AxiomPrimitive::Dependency, AxiomPrimitive::TrustRelation];
    let matches = engine.match_primitives(&dep_prims);
    if !matches.is_empty() {
        tests.push(Test::pass("Dependency+trust matches success pattern"));
    } else {
        tests.push(Test::fail("Success pattern match", "no match"));
    }

    // Test 6: summary format
    let s = engine.summary();
    if s.contains("patterns:") && s.contains("anti=") && s.contains("success=") {
        tests.push(Test::pass("Summary format correct"));
    } else {
        tests.push(Test::fail("Summary format", s));
    }

    // Results
    println!();
    let total = tests.len();
    let passed = tests.iter().filter(|t| t.passed).count();
    let failed = total - passed;

    for t in &tests {
        if t.passed {
            println!("  PASS  {}", t.name);
        } else {
            println!("  FAIL  {}", t.name);
            if let Some(n) = &t.notes {
                println!("           {}", n);
            }
        }
    }

    println!("\n=== Results: {}/{} passed ===", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        std::process::exit(1);
    }
}
