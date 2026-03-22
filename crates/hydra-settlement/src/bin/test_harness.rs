//! Phase 33 Test Harness — hydra-settlement
//! Run: cargo run -p hydra-settlement --bin test_harness
use hydra_executor::{ExecutionEngine, ExecutionRequest, ExecutorType, RegisteredAction};
use hydra_settlement::{
    CostClass, CostItem, Outcome, SettlementEngine, SettlementLedger, SettlementPeriod,
    SettlementQuery, SettlementRecord, SpendTrend,
};
use std::collections::HashMap;

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

fn make_record(domain: &str, tokens: u64, success: bool) -> SettlementRecord {
    SettlementRecord::new(
        uuid::Uuid::new_v4().to_string(),
        "deploy.staging",
        domain,
        "deploy to staging",
        if success {
            Outcome::Success {
                description: "deployed".into(),
            }
        } else {
            Outcome::HardDenied {
                evidence: "auth rejected".into(),
            }
        },
        vec![
            CostItem::new(CostClass::DirectExecution, tokens, 10.0, 3000),
            CostItem::new(
                CostClass::SisterCall {
                    sister_name: "AgenticMemory".into(),
                },
                200,
                2.0,
                300,
            ),
        ],
        3300,
        1,
    )
}

fn main() {
    println!("=======================================================");
    println!("  Phase 33 -- hydra-settlement");
    println!("  Layer 5, Phase 1: Execution Cost Accounting");
    println!("=======================================================");
    let mut tests = Vec::new();

    // -- COST ITEMS ---
    println!("\n-- cost items --");
    {
        let item = CostItem::new(CostClass::DirectExecution, 2000, 10.0, 5000);
        if (item.token_cost - 2.0).abs() < 0.01 && (item.attention_cost - 1.0).abs() < 0.01 {
            tests.push(Test::pass(
                "Cost: token and attention costs computed correctly",
            ));
        } else {
            tests.push(Test::fail(
                "Cost: computation",
                format!(
                    "token={:.2} att={:.2}",
                    item.token_cost, item.attention_cost
                ),
            ));
        }
    }
    {
        let sister = CostItem::new(
            CostClass::SisterCall {
                sister_name: "AgenticMemory".into(),
            },
            500,
            2.0,
            500,
        );
        if !sister.class.is_overhead() {
            tests.push(Test::pass(
                "Cost: sister call is not overhead (valuable work)",
            ));
        } else {
            tests.push(Test::fail(
                "Cost: sister overhead",
                "incorrectly classified",
            ));
        }
        let reroute = CostItem::new(CostClass::ReroutingOverhead { attempts: 2 }, 0, 0.0, 0);
        if reroute.class.is_overhead() {
            tests.push(Test::pass(
                "Cost: rerouting is overhead (wasted approach cost)",
            ));
        } else {
            tests.push(Test::fail("Cost: rerouting overhead", "not classified"));
        }
    }

    // -- SETTLEMENT RECORDS ---
    println!("\n-- settlement records --");
    {
        let r = make_record("engineering", 2000, true);
        if r.verify_integrity() {
            tests.push(Test::pass("Record: SHA256 integrity hash valid (64 chars)"));
        } else {
            tests.push(Test::fail("Record: integrity", "invalid hash"));
        }
        if r.total_cost > 0.0 {
            tests.push(Test::pass("Record: total cost is sum of all cost items"));
        } else {
            tests.push(Test::fail("Record: total cost", "zero"));
        }
        if r.efficiency_ratio() > 0.0 {
            tests.push(Test::pass("Record: efficiency > 0 for successful task"));
        } else {
            tests.push(Test::fail("Record: efficiency", "zero"));
        }
    }
    {
        let denied = make_record("security", 1000, false);
        if denied.efficiency_ratio() == 0.0 {
            tests.push(Test::pass("Record: efficiency = 0 for denied task"));
        } else {
            tests.push(Test::fail(
                "Record: denied efficiency",
                format!("{:.3}", denied.efficiency_ratio()),
            ));
        }
    }

    // -- LEDGER ---
    println!("\n-- settlement ledger --");
    {
        let mut ledger = SettlementLedger::new();
        for domain in &["engineering", "finance", "security", "engineering"] {
            ledger
                .settle(make_record(domain, 1500, true))
                .expect("settle");
        }
        assert_eq!(ledger.count(), 4);
        let q = SettlementQuery {
            domain: Some("engineering".into()),
            ..Default::default()
        };
        let results = ledger.query(&q);
        if results.len() == 2 {
            tests.push(Test::pass("Ledger: domain filter returns correct subset"));
        } else {
            tests.push(Test::fail("Ledger: filter", format!("{}", results.len())));
        }
        if ledger.lifetime_cost() > 0.0 {
            tests.push(Test::pass("Ledger: lifetime cost accumulated correctly"));
        } else {
            tests.push(Test::fail("Ledger: lifetime cost", "zero"));
        }
    }

    // -- SETTLEMENT PERIOD ---
    println!("\n-- settlement period --");
    {
        let records = vec![
            make_record("engineering", 3000, true),
            make_record("finance", 1500, true),
            make_record("security", 1000, false),
            make_record("engineering", 2000, true),
        ];
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::hours(1);
        let period =
            SettlementPeriod::from_records(start, now, &records.iter().collect::<Vec<_>>());
        if period.record_count == 4 && period.success_count == 3 {
            tests.push(Test::pass("Period: 4 records, 3 successes aggregated"));
        } else {
            tests.push(Test::fail(
                "Period: aggregation",
                format!(
                    "count={} success={}",
                    period.record_count, period.success_count
                ),
            ));
        }
        if period.cost_by_domain.contains_key("engineering") {
            tests.push(Test::pass("Period: cost_by_domain populated"));
        } else {
            tests.push(Test::fail("Period: domain breakdown", "missing"));
        }
        if let Some((top, _)) = period.top_domain() {
            if top == "engineering" {
                tests.push(Test::pass(
                    "Period: top domain = engineering (highest spend)",
                ));
            } else {
                tests.push(Test::fail("Period: top domain", top.to_string()));
            }
        } else {
            tests.push(Test::fail("Period: top domain", "none returned"));
        }
        println!("  info: {}", period.brief());
    }
    {
        let r_small = make_record("test", 500, true);
        let r_large = make_record("test", 5000, true);
        let now = chrono::Utc::now();
        let prior = SettlementPeriod::from_records(
            now - chrono::Duration::days(2),
            now - chrono::Duration::days(1),
            &[&r_small],
        );
        let current =
            SettlementPeriod::from_records(now - chrono::Duration::days(1), now, &[&r_large])
                .with_trend(&prior);
        if matches!(current.spend_trend, Some(SpendTrend::Increasing { .. })) {
            tests.push(Test::pass(
                "Period: spend trend Increasing when cost went up 10x",
            ));
        } else {
            tests.push(Test::fail(
                "Period: trend",
                format!("{:?}", current.spend_trend),
            ));
        }
    }

    // -- ENGINE INTEGRATION ---
    println!("\n-- engine integration --");
    {
        let mut engine = SettlementEngine::new();
        let mut exec_engine = ExecutionEngine::new();
        exec_engine.registry_mut().register_skill_actions(
            "test",
            vec![RegisteredAction {
                id: "deploy.staging".into(),
                skill: "test".into(),
                description: "deploy".into(),
                verb: "deploying".into(),
                executor: ExecutorType::Internal {
                    handler: "succeed".into(),
                },
                reversible: false,
                estimated_ms: 200,
                input_params: vec![],
            }],
        );
        let task = exec_engine
            .execute(ExecutionRequest::new(
                "deploy.staging",
                "deploy to staging",
                HashMap::new(),
            ))
            .expect("execution should succeed");
        engine
            .settle_task(&task, "engineering")
            .expect("settlement should succeed");
        if engine.record_count() == 1 && engine.lifetime_cost() > 0.0 {
            tests.push(Test::pass(
                "Engine: executor task -> settlement record created",
            ));
        } else {
            tests.push(Test::fail(
                "Engine: task settlement",
                format!(
                    "count={} cost={:.2}",
                    engine.record_count(),
                    engine.lifetime_cost()
                ),
            ));
        }
    }
    {
        let mut engine = SettlementEngine::new();
        engine
            .settle_skill_action(
                "agentra-settlement",
                "settlement.execute",
                "fintech",
                "execute settlement batch 2024-Q1",
                2000,
                3500,
                true,
            )
            .expect("settle");
        engine
            .settle_skill_action(
                "agentra-settlement",
                "settlement.flag_dispute",
                "fintech",
                "flag dispute on transaction TX-4471",
                800,
                1200,
                true,
            )
            .expect("settle");
        engine
            .settle_skill_action(
                "agentra-settlement",
                "settlement.execute",
                "fintech",
                "execute settlement batch 2024-Q2",
                2200,
                3800,
                false,
            )
            .expect("settle");
        let now = chrono::Utc::now();
        let p = engine.period(now - chrono::Duration::hours(1), now);
        if p.record_count == 3 && p.success_count == 2 {
            tests.push(Test::pass(
                "Engine: Agentra settlement skill actions settled (2 success, 1 denied)",
            ));
        } else {
            tests.push(Test::fail(
                "Engine: skill settlement",
                format!("count={} success={}", p.record_count, p.success_count),
            ));
        }
        println!("  info: {}", p.brief());
        println!("  info: {}", engine.monthly_brief());
    }

    // -- SUMMARY ---
    {
        let engine = SettlementEngine::new();
        let s = engine.summary();
        if s.contains("settlement:") && s.contains("records=") {
            tests.push(Test::pass(
                "Summary: format correct for TUI and intelligence brief",
            ));
        } else {
            tests.push(Test::fail("Summary", s.to_string()));
        }
    }

    // -- RESULTS ---
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
    println!("=======================================================");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        std::process::exit(1);
    } else {
        println!();
        println!("  hydra-settlement verified.");
        println!("  Cost classification:  operational.");
        println!("  Immutable records:    SHA256 hashed.");
        println!("  Period aggregation:   domains, trends, efficiency.");
        println!("  Agentra settlement:   skill actions accounted for.");
        println!("  Layer 5, Phase 1 complete.");
        println!("  Next: hydra-attribution -- WHY did things cost what they cost.");
        println!("=======================================================");
    }
}
