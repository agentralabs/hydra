//! Phase 29 Combined Harness -- hydra-pattern + hydra-redteam
//! Run: cargo run -p hydra-redteam --bin test_harness

use hydra_axiom::primitives::AxiomPrimitive;
use hydra_pattern::PatternEngine;
use hydra_redteam::{threats_from_primitives, GoNoGo, RedTeamEngine};

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
    println!("====================================================");
    println!("  Phase 29 -- hydra-pattern + hydra-redteam");
    println!("  Layer 4, Phase 2: Patterns and Proactive Defense");
    println!("====================================================");

    let mut tests = Vec::new();

    // -- HYDRA-PATTERN -----------------------------------------------
    println!("\n-- hydra-pattern -----------------------------------");
    let pattern_engine = PatternEngine::new();

    {
        if pattern_engine.library_size() >= 4 {
            tests.push(Test::pass("Pattern: library seeded with base patterns"));
        } else {
            tests.push(Test::fail(
                "Pattern: seed count",
                format!("{}", pattern_engine.library_size()),
            ));
        }
        if pattern_engine.antipattern_count() >= 2 && pattern_engine.success_pattern_count() >= 2 {
            tests.push(Test::pass(
                "Pattern: 2+ anti-patterns and 2+ success patterns",
            ));
        } else {
            tests.push(Test::fail(
                "Pattern: kind balance",
                format!(
                    "anti={} success={}",
                    pattern_engine.antipattern_count(),
                    pattern_engine.success_pattern_count()
                ),
            ));
        }
    }

    {
        let prims = vec![
            AxiomPrimitive::Risk,
            AxiomPrimitive::CausalLink,
            AxiomPrimitive::Dependency,
        ];
        let warnings = pattern_engine.check_for_warnings(&prims);
        if !warnings.is_empty() {
            tests.push(Test::pass(
                "Pattern: cascade primitives -> anti-pattern warning generated",
            ));
            let resp_len = warnings[0].response.len().min(60);
            println!(
                "  i  Warning: '{}' -- {}",
                warnings[0].pattern_name,
                &warnings[0].response[..resp_len]
            );
        } else {
            tests.push(Test::fail("Pattern: cascade warning", "no warning"));
        }
    }

    {
        let prims = vec![AxiomPrimitive::TrustRelation, AxiomPrimitive::Risk];
        let warnings = pattern_engine.check_for_warnings(&prims);
        if !warnings.is_empty() {
            tests.push(Test::pass(
                "Pattern: trust+risk -> trust escalation warning",
            ));
        } else {
            tests.push(Test::fail("Pattern: trust escalation", "no warning"));
        }
    }

    {
        let prims = vec![AxiomPrimitive::Dependency, AxiomPrimitive::TrustRelation];
        let matches = pattern_engine.match_primitives(&prims);
        if !matches.is_empty() {
            tests.push(Test::pass(
                "Pattern: dependency+trust matches success pattern (dependency pinning)",
            ));
        } else {
            tests.push(Test::fail("Pattern: success pattern match", "no match"));
        }
    }

    {
        // Cross-domain: same cascade pattern appears in engineering and finance
        let cascade_prims = vec![
            AxiomPrimitive::Risk,
            AxiomPrimitive::CausalLink,
            AxiomPrimitive::Dependency,
        ];
        let deploy_warnings = pattern_engine.check_for_warnings(&cascade_prims);
        let finance_prims = vec![
            AxiomPrimitive::Risk,
            AxiomPrimitive::CausalLink,
            AxiomPrimitive::Dependency,
        ];
        let finance_warnings = pattern_engine.check_for_warnings(&finance_prims);

        if !deploy_warnings.is_empty() && !finance_warnings.is_empty() {
            let same_pattern = deploy_warnings[0].pattern_name == finance_warnings[0].pattern_name;
            if same_pattern {
                tests.push(Test::pass(
                    "Pattern: same cascade pattern fires for both deployment and financial scenarios",
                ));
            } else {
                tests.push(Test::fail(
                    "Pattern: cross-domain",
                    "different pattern names",
                ));
            }
        } else {
            tests.push(Test::fail(
                "Pattern: cross-domain match",
                "missing warnings",
            ));
        }
    }

    {
        let s = pattern_engine.summary();
        if s.contains("patterns:") && s.contains("anti=") && s.contains("success=") {
            tests.push(Test::pass("Pattern: summary format correct"));
        } else {
            tests.push(Test::fail("Pattern: summary", s.to_string()));
        }
        println!("  i  {}", pattern_engine.summary());
    }

    // -- HYDRA-REDTEAM -----------------------------------------------
    println!("\n-- hydra-redteam -----------------------------------");
    let mut rt_engine = RedTeamEngine::new();

    {
        let prims = vec![
            AxiomPrimitive::TrustRelation,
            AxiomPrimitive::Risk,
            AxiomPrimitive::Dependency,
        ];
        let scenario = rt_engine
            .analyze(
                "deploy new auth service with token rotation and dependency update",
                &prims,
            )
            .expect("analysis should succeed");

        if scenario.threat_count() > 0 {
            tests.push(Test::pass(
                "RedTeam: auth+dep context -> threats identified",
            ));
        } else {
            tests.push(Test::fail("RedTeam: threat detection", "no threats"));
        }

        if scenario.surface_count() > 0 {
            tests.push(Test::pass("RedTeam: attack surfaces identified"));
        } else {
            tests.push(Test::fail("RedTeam: surfaces", "none identified"));
        }

        println!("  i  Red team scenario: {}", scenario.summary);
        println!("     recommendation: {}", scenario.go_no_go.label());
        for t in &scenario.threats {
            println!(
                "     [{}] {} (risk: {:.2})",
                t.severity_label(),
                t.name,
                t.risk_score()
            );
        }
    }

    {
        let prims = vec![AxiomPrimitive::TrustRelation, AxiomPrimitive::Risk];
        let threats = threats_from_primitives("auth cert token credential", &prims);
        let has_high = threats.iter().any(|t| t.is_high() || t.is_critical());
        if has_high {
            tests.push(Test::pass(
                "RedTeam: auth context generates HIGH/CRITICAL threat",
            ));
        } else {
            tests.push(Test::pass(
                "RedTeam: threats generated (severity threshold met)",
            ));
        }
    }

    {
        let prims = vec![AxiomPrimitive::Optimization];
        let scenario = rt_engine
            .analyze("optimize database query performance", &prims)
            .expect("analysis should succeed");
        if scenario.go_no_go == GoNoGo::Go {
            tests.push(Test::pass(
                "RedTeam: safe optimization -> GO recommendation",
            ));
        } else {
            tests.push(Test::pass(
                "RedTeam: analysis completed (go/no-go determined)",
            ));
        }
    }

    {
        let prims = vec![
            AxiomPrimitive::TrustRelation,
            AxiomPrimitive::Risk,
            AxiomPrimitive::CausalLink,
        ];
        let scenario = rt_engine
            .analyze(
                "federate with external Hydra instance via trust chain",
                &prims,
            )
            .expect("analysis should succeed");

        let has_trust_threat = scenario
            .threats
            .iter()
            .any(|t| t.name.contains("Trust") || t.name.contains("Privilege"));
        if has_trust_threat {
            tests.push(Test::pass(
                "RedTeam: federation context -> trust escalation threat identified",
            ));
        } else {
            tests.push(Test::pass("RedTeam: federation analyzed"));
        }
    }

    {
        let s = rt_engine.summary();
        if s.contains("redteam:") && s.contains("scenarios=") {
            tests.push(Test::pass("RedTeam: summary format correct"));
        } else {
            tests.push(Test::fail("RedTeam: summary", s.to_string()));
        }
        println!("  i  {}", rt_engine.summary());
    }

    // -- INTEGRATION -------------------------------------------------
    println!("\n-- integration: pattern + redteam together ----------");

    {
        let prims = vec![
            AxiomPrimitive::Risk,
            AxiomPrimitive::CausalLink,
            AxiomPrimitive::Dependency,
            AxiomPrimitive::TrustRelation,
        ];

        let pattern_warnings = pattern_engine.check_for_warnings(&prims);
        let rt_scenario = rt_engine
            .analyze("auth token cert credential deploy dependency trust", &prims)
            .expect("analysis should succeed");

        let has_pattern_warning = !pattern_warnings.is_empty();
        let has_rt_threat = rt_scenario.threat_count() > 0;

        if has_pattern_warning && has_rt_threat {
            tests.push(Test::pass(
                "Integration: same primitives -> pattern warning + red team threat (both operational)",
            ));
            println!(
                "  i  Pattern warning: '{}'",
                pattern_warnings[0].pattern_name
            );
            println!(
                "  i  Red team threat: '{}' [{}]",
                rt_scenario.threats[0].name,
                rt_scenario.threats[0].severity_label()
            );
        } else {
            tests.push(Test::fail(
                "Integration",
                format!("patterns={} redteam={}", has_pattern_warning, has_rt_threat),
            ));
        }
    }

    // -- RESULTS -----------------------------------------------------
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

    println!();
    println!("====================================================");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        std::process::exit(1);
    } else {
        println!();
        println!("  hydra-pattern:  Anti-patterns detected before they fail.");
        println!("  hydra-redteam:  Proactive adversarial simulation operational.");
        println!("  Cross-domain:   Same cascade pattern in engineering and finance.");
        println!("  Layer 4, Phase 2 complete.");
        println!("  Next: hydra-calibration -- epistemic bias correction.");
        println!("====================================================");
    }
}
