//! Phase 30 Test Harness — hydra-calibration
//! Run: cargo run -p hydra-calibration --bin test_harness

use hydra_calibration::{
    constants::{MIN_RECORDS_FOR_BIAS, SIGNIFICANT_BIAS_THRESHOLD},
    CalibrationEngine, JudgmentType,
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

fn build_engine(domain: &str, stated: f64, actual: f64, n: usize) -> CalibrationEngine {
    let mut engine = CalibrationEngine::new();
    let ids: Vec<String> = (0..n)
        .map(|_| {
            engine
                .record_prediction(domain, JudgmentType::RiskAssessment, stated)
                .unwrap()
        })
        .collect();
    for id in &ids {
        engine.record_outcome(id, actual).unwrap();
    }
    engine
}

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  Phase 30 — hydra-calibration");
    println!("  Layer 4, Phase 4: Epistemic Calibration");
    println!("  \"I know where my judgment goes wrong.\"");
    println!("═══════════════════════════════════════════════════════");

    let mut tests = Vec::new();

    // -- RECORD KEEPING ---------------------------------------------------
    println!("\n── record keeping ───────────────────────────────────");

    {
        let mut engine = CalibrationEngine::new();
        let id = engine
            .record_prediction("engineering", JudgmentType::RiskAssessment, 0.85)
            .unwrap();
        assert_eq!(engine.record_count(), 1);
        engine.record_outcome(&id, 0.70).unwrap();

        let resolved = engine.resolved_records();
        if resolved.len() == 1 {
            let record = resolved[0];
            let offset = record.offset().unwrap();
            // 0.70 - 0.85 = -0.15 (overconfident)
            if (offset - (-0.15)).abs() < 1e-10 && record.is_overconfident() {
                tests.push(Test::pass(
                    "Record: offset = actual - stated = -0.15 (overconfident)",
                ));
            } else {
                tests.push(Test::fail("Record: offset", format!("{:.4}", offset)));
            }
        } else {
            tests.push(Test::fail(
                "Record: resolved count",
                format!("{}", resolved.len()),
            ));
        }
    }

    // -- BIAS DETECTION ---------------------------------------------------
    println!("\n── bias detection ───────────────────────────────────");

    {
        // Consistent overconfidence in fintech risk
        let engine = build_engine("fintech", 0.88, 0.65, MIN_RECORDS_FOR_BIAS);
        if engine.significant_bias_count() > 0 {
            tests.push(Test::pass("Bias: overconfidence detected after 10 records"));
        } else {
            tests.push(Test::fail(
                "Bias: overconfidence detection",
                format!("biases={}", engine.significant_bias_count()),
            ));
        }
    }

    {
        // Consistent underconfidence in security
        let mut engine = CalibrationEngine::new();
        let ids: Vec<String> = (0..MIN_RECORDS_FOR_BIAS)
            .map(|_| {
                engine
                    .record_prediction("security", JudgmentType::SecurityAssessment, 0.50)
                    .unwrap()
            })
            .collect();
        for id in &ids {
            engine.record_outcome(id, 0.78).unwrap(); // actual >> stated
        }
        if engine.significant_bias_count() > 0 {
            tests.push(Test::pass("Bias: underconfidence detected (security)"));
        } else {
            tests.push(Test::fail("Bias: underconfidence", "not detected"));
        }
    }

    {
        // Well-calibrated — tiny offset, not significant
        let engine = build_engine("general", 0.75, 0.77, MIN_RECORDS_FOR_BIAS);
        if engine.significant_bias_count() == 0 {
            tests.push(Test::pass(
                "Bias: tiny offset (0.02) -> no significant bias",
            ));
        } else {
            tests.push(Test::fail(
                "Bias: well-calibrated false positive",
                "bias detected",
            ));
        }
    }

    // -- CONFIDENCE ADJUSTMENT --------------------------------------------
    println!("\n── confidence adjustment ────────────────────────────");

    {
        let engine = build_engine("fintech", 0.88, 0.65, MIN_RECORDS_FOR_BIAS);

        let adjusted = engine.calibrate(0.85, "fintech", &JudgmentType::RiskAssessment);
        if adjusted.calibrated < adjusted.raw {
            tests.push(Test::pass(
                "Adjust: overconfident domain -> calibrated < raw",
            ));
            println!(
                "  Fintech risk: raw={:.2} -> calibrated={:.2} (bias={:+.2})",
                adjusted.raw, adjusted.calibrated, adjusted.bias_applied
            );
        } else {
            tests.push(Test::fail(
                "Adjust: direction",
                format!(
                    "raw={:.2} calibrated={:.2}",
                    adjusted.raw, adjusted.calibrated
                ),
            ));
        }

        if adjusted.is_reliable {
            tests.push(Test::pass(
                "Adjust: is_reliable=true (enough calibration data)",
            ));
        } else {
            tests.push(Test::fail("Adjust: reliability flag", "not reliable"));
        }

        if adjusted.changed_significantly() {
            tests.push(Test::pass("Adjust: changed_significantly() = true"));
        } else {
            tests.push(Test::fail("Adjust: significance", "no significant change"));
        }
    }

    {
        // Unknown domain -> returns raw, not reliable
        let engine = CalibrationEngine::new();
        let adjusted = engine.calibrate(0.75, "unknown-domain", &JudgmentType::RiskAssessment);
        if adjusted.calibrated == adjusted.raw && !adjusted.is_reliable {
            tests.push(Test::pass(
                "Adjust: unknown domain -> raw returned, not reliable",
            ));
        } else {
            tests.push(Test::fail(
                "Adjust: unknown domain",
                format!(
                    "calibrated={:.2} reliable={}",
                    adjusted.calibrated, adjusted.is_reliable
                ),
            ));
        }
    }

    // -- CALIBRATION HEALTH -----------------------------------------------
    println!("\n── calibration health ───────────────────────────────");

    {
        let perfect = CalibrationEngine::new();
        if (perfect.calibration_health() - 1.0).abs() < 1e-10 {
            tests.push(Test::pass(
                "Health: no data -> health = 1.0 (no known biases)",
            ));
        } else {
            tests.push(Test::fail(
                "Health: empty engine",
                format!("{:.2}", perfect.calibration_health()),
            ));
        }
    }

    {
        let biased = build_engine("engineering", 0.90, 0.60, MIN_RECORDS_FOR_BIAS);
        let health = biased.calibration_health();
        if health < 1.0 {
            tests.push(Test::pass("Health: large bias (-0.30) -> health < 1.0"));
        } else {
            tests.push(Test::fail(
                "Health: biased engine",
                format!("{:.2}", health),
            ));
        }
        println!("  Calibration health with -0.30 bias: {:.2}", health);
    }

    // -- MULTI-DOMAIN CALIBRATION -----------------------------------------
    println!("\n── multi-domain calibration ─────────────────────────");

    {
        let mut engine = CalibrationEngine::new();

        // Build calibration data for 3 domains
        let domains: Vec<(&str, JudgmentType, f64, f64)> = vec![
            ("fintech", JudgmentType::RiskAssessment, 0.88, 0.65),
            ("security", JudgmentType::SecurityAssessment, 0.50, 0.78),
            ("engineering", JudgmentType::ComplexityEstimate, 0.70, 0.72),
        ];

        for (domain, jtype, stated, actual) in &domains {
            let ids: Vec<String> = (0..MIN_RECORDS_FOR_BIAS)
                .map(|_| {
                    engine
                        .record_prediction(*domain, jtype.clone(), *stated)
                        .unwrap()
                })
                .collect();
            for id in &ids {
                engine.record_outcome(id, *actual).unwrap();
            }
        }

        if engine.profile_count() == 3 {
            tests.push(Test::pass("Multi: 3 domain bias profiles built"));
        } else {
            tests.push(Test::fail(
                "Multi: profile count",
                format!("{}", engine.profile_count()),
            ));
        }

        // Fintech should be overconfident, security underconfident
        let fin_adj = engine.calibrate(0.85, "fintech", &JudgmentType::RiskAssessment);
        let sec_adj = engine.calibrate(0.55, "security", &JudgmentType::SecurityAssessment);

        if fin_adj.calibrated < fin_adj.raw {
            tests.push(Test::pass("Multi: fintech risk -> calibrated downward"));
        } else {
            tests.push(Test::fail("Multi: fintech", "not downward"));
        }

        if sec_adj.calibrated > sec_adj.raw {
            tests.push(Test::pass("Multi: security -> calibrated upward"));
        } else {
            tests.push(Test::fail("Multi: security", "not upward"));
        }

        println!("  Multi-domain calibration:");
        println!(
            "     fintech risk:     raw={:.2} -> calibrated={:.2} ({:+.2})",
            fin_adj.raw, fin_adj.calibrated, fin_adj.bias_applied
        );
        println!(
            "     security:         raw={:.2} -> calibrated={:.2} ({:+.2})",
            sec_adj.raw, sec_adj.calibrated, sec_adj.bias_applied
        );
        println!("  {}", engine.summary());
    }

    // -- SUMMARY ----------------------------------------------------------
    {
        let engine = CalibrationEngine::new();
        let s = engine.summary();
        if s.contains("calibration:") && s.contains("health=") {
            tests.push(Test::pass("Summary: format correct for TUI display"));
        } else {
            tests.push(Test::fail("Summary", s));
        }
    }

    // -- RESULTS ----------------------------------------------------------
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
        std::process::exit(1);
    } else {
        println!();
        println!("  hydra-calibration verified.");
        println!("  Overconfidence: detected and corrected downward.");
        println!("  Underconfidence: detected and corrected upward.");
        println!("  Multi-domain: each domain has its own bias profile.");
        println!("  Health score: honest accounting of calibration quality.");
        println!("  \"My raw confidence is 0.83. Calibrated: 0.72.\"");
        println!("  Layer 4, Phase 4 complete.");
        println!("  Next: hydra-oracle — probabilistic futures.");
        println!("═══════════════════════════════════════════════════════");
    }
}

// Unused import guard: SIGNIFICANT_BIAS_THRESHOLD is used in the spec for
// completeness but the harness tests it indirectly via engine methods.
const _: f64 = SIGNIFICANT_BIAS_THRESHOLD;
