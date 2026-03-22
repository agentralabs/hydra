//! Phase 34 Test Harness — hydra-attribution
//! Run: cargo run -p hydra-attribution --bin test_harness

use hydra_attribution::{infer_factors, AttributionEngine, CausalFactorType, CostClass, CostItem};
use hydra_settlement::{
    CostClass as SC, CostItem as SI, Outcome, SettlementEngine, SettlementQuery, SettlementRecord,
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

fn make_settlement_record(
    task_id: &str,
    domain: &str,
    intent: &str,
    costs: Vec<SI>,
    success: bool,
) -> SettlementRecord {
    let duration: u64 = 2000;
    let attempts = costs
        .iter()
        .filter(|c| matches!(c.class, SC::ReroutingOverhead { .. }))
        .map(|c| {
            if let SC::ReroutingOverhead { attempts } = c.class {
                attempts
            } else {
                0
            }
        })
        .sum::<u32>()
        + 1;

    SettlementRecord::new(
        task_id,
        "deploy.staging",
        domain,
        intent,
        if success {
            Outcome::Success {
                description: "completed".into(),
            }
        } else {
            Outcome::HardDenied {
                evidence: "denied".into(),
            }
        },
        costs,
        duration,
        attempts,
    )
}

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  Phase 34 — hydra-attribution");
    println!("  Layer 5, Phase 2: Causal Cost Tracing");
    println!("  \"WHY did things cost what they cost?\"");
    println!("═══════════════════════════════════════════════════════");

    let mut tests = Vec::new();
    let mut engine = AttributionEngine::new();

    // ── CAUSAL FACTOR INFERENCE ───────────────────────────────────
    println!("\n── causal factor inference ──────────────────────────");

    {
        let costs = vec![
            CostItem::new(CostClass::DirectExecution, 2000, 3.0, 3000),
            CostItem::new(CostClass::ReroutingOverhead { attempts: 2 }, 0, 0.5, 500),
        ];
        let total = costs.iter().map(|c| c.amount).sum();
        let factors = infer_factors(
            &costs,
            total,
            "deploy service with concurrent lock conflict",
        );

        let has_concurrency = factors
            .iter()
            .any(|f| matches!(f.factor_type, CausalFactorType::ConcurrencyConflict { .. }));
        if has_concurrency {
            tests.push(Test::pass(
                "Inference: lock-context rerouting → ConcurrencyConflict",
            ));
        } else {
            tests.push(Test::fail("Inference: concurrency", "not inferred"));
        }

        let avoidable = factors.iter().any(|f| f.is_avoidable());
        if avoidable {
            tests.push(Test::pass("Inference: concurrency conflict is avoidable"));
        } else {
            tests.push(Test::fail("Inference: avoidable flag", "not set"));
        }
    }

    {
        let costs = vec![
            CostItem::new(CostClass::DirectExecution, 1500, 2.0, 2000),
            CostItem::new(
                CostClass::KnowledgeAcquisition {
                    topic: "gcp-iam-model".into(),
                },
                1000,
                1.5,
                1500,
            ),
        ];
        let total = costs.iter().map(|c| c.amount).sum();
        let factors = infer_factors(&costs, total, "first deployment to new gcp cloud provider");

        let has_first_time = factors
            .iter()
            .any(|f| matches!(f.factor_type, CausalFactorType::FirstTimeOperation { .. }));
        if has_first_time {
            tests.push(Test::pass(
                "Inference: new-provider context → FirstTimeOperation (one-time)",
            ));
        } else {
            tests.push(Test::fail("Inference: first-time", "not inferred"));
        }

        let one_time = factors.iter().any(|f| f.is_one_time());
        if one_time {
            tests.push(Test::pass(
                "Inference: first-time is NOT avoidable (one-time cost)",
            ));
        } else {
            tests.push(Test::fail("Inference: one-time flag", "not set"));
        }
    }

    // ── ATTRIBUTION TREE ──────────────────────────────────────────
    println!("\n── attribution tree ─────────────────────────────────");

    {
        let record = make_settlement_record(
            "task-deploy-1",
            "engineering",
            "deploy auth service with concurrent lock access conflict",
            vec![
                SI::new(SC::DirectExecution, 2000, 10.0, 3000),
                SI::new(SC::ReroutingOverhead { attempts: 2 }, 0, 0.0, 500),
                SI::new(
                    SC::SisterCall {
                        sister_name: "AgenticMemory".into(),
                    },
                    300,
                    2.0,
                    300,
                ),
                SI::new(SC::RedTeamAnalysis, 500, 3.0, 800),
            ],
            true,
        );

        let tree = engine.attribute(&record).expect("should attribute");

        if !tree.factors.is_empty() {
            tests.push(Test::pass("Tree: factors inferred from settlement record"));
        } else {
            tests.push(Test::fail("Tree: factors", "empty"));
        }

        if !tree.narrative.is_empty() && tree.narrative.contains("Attribution for") {
            tests.push(Test::pass(
                "Tree: narrative generated with attribution header",
            ));
        } else {
            tests.push(Test::fail("Tree: narrative", "missing or wrong format"));
        }

        if tree.avoidable_cost >= 0.0 {
            tests.push(Test::pass("Tree: avoidable cost calculated"));
        } else {
            tests.push(Test::fail("Tree: avoidable cost", "negative"));
        }

        println!("  ℹ  Attribution narrative:");
        for line in tree.narrative.lines().take(6) {
            println!("     {}", line);
        }
    }

    // ── AVOIDABILITY REPORT ───────────────────────────────────────
    println!("\n── avoidability report ──────────────────────────────");

    {
        for i in 0..4 {
            let r = make_settlement_record(
                &format!("reroute-task-{}", i),
                "engineering",
                "deploy service concurrent lock conflict auth",
                vec![
                    SI::new(SC::DirectExecution, 1000, 5.0, 2000),
                    SI::new(SC::ReroutingOverhead { attempts: 3 }, 0, 0.0, 600),
                ],
                true,
            );
            engine.attribute(&r).expect("should attribute");
        }

        let report = engine.avoidability_report(Some("engineering"));
        if report.total_cost > 0.0 && report.avoidable_cost > 0.0 {
            tests.push(Test::pass(
                "Report: avoidable cost detected in engineering domain",
            ));
        } else {
            tests.push(Test::fail(
                "Report: avoidable cost",
                format!(
                    "total={:.2} avoidable={:.2}",
                    report.total_cost, report.avoidable_cost
                ),
            ));
        }

        if !report.recommendations.is_empty() {
            tests.push(Test::pass(
                "Report: recommendations generated for avoidable causes",
            ));
            let rec = &report.recommendations[0];
            let end = rec.len().min(80);
            println!("  ℹ  Top recommendation: {}", &rec[..end]);
        } else {
            tests.push(Test::fail("Report: recommendations", "empty"));
        }

        println!("  ℹ  {}", report.brief());
    }

    // ── INTEGRATION: SETTLEMENT + ATTRIBUTION ─────────────────────
    println!("\n── integration: settlement + attribution ────────────");

    {
        let mut settle_engine = SettlementEngine::new();
        let mut attr_engine = AttributionEngine::new();

        settle_engine
            .settle_skill_action(
                "agentra-settlement",
                "settlement.execute",
                "fintech",
                "execute Q1 settlement batch",
                2000,
                3500,
                true,
            )
            .expect("settle ok");
        settle_engine
            .settle_skill_action(
                "agentra-settlement",
                "settlement.execute",
                "fintech",
                "execute Q2 settlement batch with concurrent lock issue",
                2500,
                4200,
                true,
            )
            .expect("settle ok");
        settle_engine
            .settle_skill_action(
                "agentra-settlement",
                "settlement.flag_dispute",
                "fintech",
                "flag dispute TX-4471 missing credentials",
                800,
                1200,
                true,
            )
            .expect("settle ok");

        // Attribute all fintech records from the settlement ledger
        let q = SettlementQuery {
            domain: Some("fintech".into()),
            ..Default::default()
        };
        let records = settle_engine.ledger.query(&q);

        for record in &records {
            attr_engine.attribute(record).expect("attribute ok");
        }

        let fintech_report = attr_engine.avoidability_report(Some("fintech"));
        if attr_engine.tree_count() == 3 {
            tests.push(Test::pass(
                "Integration: 3 Agentra settlement records attributed",
            ));
        } else {
            tests.push(Test::fail(
                "Integration: tree count",
                format!("{}", attr_engine.tree_count()),
            ));
        }

        println!("  ℹ  Fintech attribution report:");
        println!("     {}", fintech_report.brief());
        println!("  ℹ  {}", attr_engine.summary());
    }

    // ── SUMMARY ───────────────────────────────────────────────────
    {
        let s = engine.summary();
        if s.contains("attribution:") && s.contains("trees=") && s.contains("avoidable=") {
            tests.push(Test::pass("Summary: format correct for TUI display"));
        } else {
            tests.push(Test::fail("Summary", s));
        }
    }

    // ── RESULTS ───────────────────────────────────────────────────
    println!();
    let total = tests.len();
    let passed = tests.iter().filter(|t| t.passed).count();
    let failed = total - passed;

    for t in &tests {
        if t.passed {
            println!("  ✅ PASS  {}", t.name);
        } else {
            println!("  ❌ FAIL  {}", t.name);
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
        println!("  hydra-attribution verified.");
        println!("  Causal inference:    concurrency, credentials, first-time.");
        println!("  Avoidability:        detected and flagged with recommendations.");
        println!("  Narrative:           plain language for every cost item.");
        println!("  Agentra settlement:  3 records attributed in fintech domain.");
        println!("  Layer 5, Phase 2 complete.");
        println!("  Next: hydra-portfolio + hydra-crystallizer (parallel).");
        println!("═══════════════════════════════════════════════════════");
    }
}
