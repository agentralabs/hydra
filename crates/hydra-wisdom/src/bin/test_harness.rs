//! Phase 32 Test Harness — THE FINAL LAYER 4 CRATE
//! Run: cargo run -p hydra-wisdom --bin test_harness

use hydra_wisdom::{
    CalibrationEvidence, OracleEvidence, PatternEvidence, RedTeamEvidence, WisdomEngine,
    WisdomInput,
};

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
    println!("═══════════════════════════════════════════════════════");
    println!("  Phase 32 — THE FINAL LAYER 4 CRATE");
    println!("  hydra-wisdom — Where Intelligence Becomes Judgment");
    println!("═══════════════════════════════════════════════════════");

    let mut tests = Vec::new();
    let mut engine = WisdomEngine::new();

    // ── PROCEED WITH CONDITIONS ───────────────────────────────────
    println!("\n── proceed with conditions ──────────────────────────");

    {
        let input = WisdomInput::new(
            "deploy auth service to production with cert rotation",
            "fintech",
        )
        .with_base_confidence(0.78)
        .with_pattern(PatternEvidence {
            pattern_name: "Trust Escalation".into(),
            is_warning: true,
            similarity: 0.75,
            response: "Audit trust scope and permissions before deployment.".into(),
        })
        .with_oracle(OracleEvidence {
            scenario_name: "partial rollback required".into(),
            probability: 0.22,
            is_adverse: true,
            intervention: Some("Use blue-green deployment with fast rollback.".into()),
        })
        .with_redteam(RedTeamEvidence {
            threat_name: "Credential Exploitation".into(),
            severity: "HIGH".into(),
            risk_score: 0.76,
            mitigation: "Rotate all credentials 30 minutes before deployment.".into(),
        })
        .with_calibration(CalibrationEvidence {
            raw_confidence: 0.78,
            calibrated_confidence: 0.64,
            bias_direction: "overconfident".into(),
            is_reliable: true,
        });

        let stmt = engine.synthesize(&input).expect("should synthesize");

        if stmt.recommendation.label() == "PROCEED-WITH-CONDITIONS" {
            tests.push(Test::pass(
                "Wisdom: high threat + pattern → PROCEED-WITH-CONDITIONS",
            ));
        } else {
            tests.push(Test::fail(
                "Wisdom: conditions recommendation",
                stmt.recommendation.label().to_string(),
            ));
        }

        if !stmt.reasoning_chain.is_empty() {
            tests.push(Test::pass(
                "Wisdom: reasoning chain populated (explains the judgment)",
            ));
        } else {
            tests.push(Test::fail("Wisdom: reasoning chain", "empty"));
        }

        if !stmt.reversal_conditions.is_empty() {
            tests.push(Test::pass(
                "Wisdom: reversal conditions stated (what would change it)",
            ));
        } else {
            tests.push(Test::fail("Wisdom: reversal conditions", "empty"));
        }

        println!("  {}", stmt.tui_summary());
        println!("  Reasoning chain:");
        for (i, r) in stmt.reasoning_chain.iter().enumerate() {
            let display_len = r.len().min(90);
            println!("     {}. {}", i + 1, &r[..display_len]);
        }
    }

    // ── DO NOT PROCEED ────────────────────────────────────────────
    println!("\n── do not proceed ───────────────────────────────────");

    {
        let input = WisdomInput::new(
            "deploy service with critical zero-day vulnerability",
            "security",
        )
        .with_base_confidence(0.80)
        .with_redteam(RedTeamEvidence {
            threat_name: "Zero Day Exploit".into(),
            severity: "CRITICAL".into(),
            risk_score: 0.96,
            mitigation: "Do not deploy until vulnerability is patched and verified.".into(),
        });

        let stmt = engine.synthesize(&input).expect("should synthesize");

        if stmt.recommendation.label() == "DO-NOT-PROCEED" {
            tests.push(Test::pass(
                "Wisdom: critical threat → DO-NOT-PROCEED (hard block)",
            ));
        } else {
            tests.push(Test::fail(
                "Wisdom: critical block",
                stmt.recommendation.label().to_string(),
            ));
        }
        println!("  {}", stmt.tui_summary());
    }

    // ── CLEAN PROCEED ─────────────────────────────────────────────
    println!("\n── clean proceed ────────────────────────────────────");

    {
        let input = WisdomInput::new("optimize database query index", "engineering")
            .with_base_confidence(0.88)
            .with_oracle(OracleEvidence {
                scenario_name: "query performance improves significantly".into(),
                probability: 0.82,
                is_adverse: false,
                intervention: None,
            })
            .with_calibration(CalibrationEvidence {
                raw_confidence: 0.88,
                calibrated_confidence: 0.86,
                bias_direction: "well-calibrated".into(),
                is_reliable: true,
            });

        let stmt = engine.synthesize(&input).expect("should synthesize");

        if stmt.recommendation.is_proceed() {
            tests.push(Test::pass(
                "Wisdom: safe optimization → PROCEED (no conditions)",
            ));
        } else {
            tests.push(Test::fail(
                "Wisdom: safe proceed",
                stmt.recommendation.label().to_string(),
            ));
        }
        println!("  {}", stmt.tui_summary());
    }

    // ── WISDOM MEMORY ─────────────────────────────────────────────
    println!("\n── wisdom memory ────────────────────────────────────");

    {
        engine.record_last_outcome(true, "optimization improved p99 latency by 42%");

        let similar = WisdomInput::new(
            "deploy auth service cert rotation production fintech",
            "fintech",
        )
        .with_pattern(PatternEvidence {
            pattern_name: "Trust Escalation".into(),
            is_warning: true,
            similarity: 0.72,
            response: "Audit trust scope.".into(),
        });

        let stmt2 = engine.synthesize(&similar).expect("should synthesize");

        let has_prior = stmt2
            .reasoning_chain
            .iter()
            .any(|r| r.contains("Prior") || r.contains("prior"));
        if has_prior {
            tests.push(Test::pass(
                "Memory: similar context → prior judgment recalled in reasoning chain",
            ));
        } else {
            tests.push(Test::fail("Memory: prior recall", "not in reasoning chain"));
        }

        if engine.memory_size() >= 3 {
            tests.push(Test::pass("Memory: all judgments stored in wisdom memory"));
        } else {
            tests.push(Test::fail(
                "Memory: size",
                format!("{}", engine.memory_size()),
            ));
        }
    }

    // ── THE LAYER 4 MILESTONE ─────────────────────────────────────
    println!("\n── the layer 4 milestone ────────────────────────────");
    println!("  \"The data says X. But pattern history says be careful.\"");
    println!("  \"Three times before when data said X it was wrong\"");
    println!("  \"because of Y. Recommend verifying Y before acting.\"");

    {
        let input = WisdomInput::new(
            "data shows deployment window is safe error rate nominal",
            "engineering",
        )
        .with_base_confidence(0.82)
        .with_pattern(PatternEvidence {
            pattern_name: "Cascade Failure".into(),
            is_warning: true,
            similarity: 0.80,
            response: "This pattern has preceded 3 cascade events. \
                           Verify downstream dependency health before proceeding."
                .into(),
        })
        .with_oracle(OracleEvidence {
            scenario_name: "cascade if downstream unhealthy".into(),
            probability: 0.35,
            is_adverse: true,
            intervention: Some("Check downstream health endpoints before deploy.".into()),
        });

        let stmt = engine.synthesize(&input).expect("should synthesize");

        if !matches!(stmt.recommendation, hydra_wisdom::Recommendation::Proceed) {
            tests.push(Test::pass(
                "Milestone: data says safe + pattern warns → \
                 NOT clean proceed (judgment, not computation)",
            ));
        } else {
            tests.push(Test::fail(
                "Milestone: pattern override",
                "clean proceed despite warning",
            ));
        }

        if stmt
            .reasoning_chain
            .iter()
            .any(|r| r.contains("Pattern match"))
        {
            tests.push(Test::pass(
                "Milestone: reasoning chain explicitly states pattern evidence",
            ));
        } else {
            tests.push(Test::fail("Milestone: pattern in reasoning", "not stated"));
        }

        println!("  {}", stmt.tui_summary());
        println!("  Key reasoning:");
        for r in stmt.reasoning_chain.iter().take(3) {
            let display_len = r.len().min(95);
            println!("     -> {}", &r[..display_len]);
        }
    }

    // ── SUMMARY ───────────────────────────────────────────────────
    {
        let s = engine.summary();
        if s.contains("wisdom:") && s.contains("judgments=") && s.contains("memories=") {
            tests.push(Test::pass("Summary: format correct for TUI display"));
        } else {
            tests.push(Test::fail("Summary", s.clone()));
        }
        println!("\n  {}", engine.summary());
    }

    // ── RESULTS ───────────────────────────────────────────────────
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
    println!("═══════════════════════════════════════════════════════");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        println!("═══════════════════════════════════════════════════════");
        std::process::exit(1);
    } else {
        println!();
        println!("  Layer 4 complete. Phase 32 verified.");
        println!("═══════════════════════════════════════════════════════");
    }
}
