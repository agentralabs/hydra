//! Phase 35+36 Combined Harness — hydra-portfolio + hydra-crystallizer
//! Run: cargo run -p hydra-crystallizer --bin test_harness

use hydra_attribution::AttributionEngine;
use hydra_crystallizer::{ArtifactKind, CrystallizationSource, CrystallizerEngine};
use hydra_portfolio::{ObjectiveCategory, PortfolioEngine, PortfolioObjective};
use hydra_settlement::{CostClass, CostItem, Outcome, SettlementEngine, SettlementRecord};

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

fn costs() -> Vec<CostItem> {
    vec![CostItem::new(CostClass::DirectExecution, 2000, 5.0, 1000)]
}
fn srec(id: &str, domain: &str) -> SettlementRecord {
    SettlementRecord::new(
        id,
        "deploy.staging",
        domain,
        "deploy service",
        Outcome::Success {
            description: "done".into(),
        },
        costs(),
        1000,
        1,
    )
}
fn frec(id: &str, domain: &str) -> SettlementRecord {
    SettlementRecord::new(
        id,
        "deploy.staging",
        domain,
        "deploy with concurrent lock",
        Outcome::HardDenied {
            evidence: "auth rejected — credentials expired".into(),
        },
        costs(),
        500,
        3,
    )
}

fn check(tests: &mut Vec<Test>, name: &'static str, ok: bool, note: &str) {
    if ok {
        tests.push(Test::pass(name));
    } else {
        tests.push(Test::fail(name, note));
    }
}

fn main() {
    println!("=======================================================");
    println!("  Phase 35+36 — hydra-portfolio + hydra-crystallizer");
    println!("  Layer 5, Phases 3 and 4: Allocation + Artifacts");
    println!("=======================================================");
    let mut tests = Vec::new();

    // ── HYDRA-PORTFOLIO ──
    println!("\n-- hydra-portfolio --");
    {
        let mut engine = PortfolioEngine::new();
        engine
            .add_objective(PortfolioObjective::new(
                "Harden settlement auth surface",
                "Red team HIGH threat",
                ObjectiveCategory::SecurityHardening,
                0.88,
                0.80,
                0.85,
                0.92,
                20.0,
            ))
            .expect("add security");
        engine
            .add_objective(PortfolioObjective::new(
                "Reduce concurrent lock overhead",
                "28% avoidable via coordination",
                ObjectiveCategory::CostReduction,
                0.10,
                0.75,
                0.60,
                0.65,
                10.0,
            ))
            .expect("add cost");
        engine
            .add_objective(PortfolioObjective::new(
                "Expand supply chain skill coverage",
                "5 recurring knowledge gaps",
                ObjectiveCategory::CapabilityExpansion,
                0.40,
                0.70,
                0.70,
                0.50,
                15.0,
            ))
            .expect("add capability");

        let alloc = engine.allocate(100.0, "Q2-2026").expect("allocate");
        check(
            &mut tests,
            "Portfolio: 3 objectives ranked and allocated",
            !alloc.allocations.is_empty(),
            "empty",
        );
        let top = alloc.top_recommendation().expect("top");
        check(
            &mut tests,
            "Portfolio: security objective ranks first (high risk + urgency)",
            top.name.contains("settlement auth"),
            &top.name,
        );
        let total_pct: f64 = alloc.allocations.iter().map(|a| a.allocated_pct).sum();
        check(
            &mut tests,
            "Portfolio: allocations sum to 100%",
            (total_pct - 100.0).abs() < 0.1,
            &format!("{:.2}", total_pct),
        );
        println!("  info: {}", alloc.brief());
        for a in &alloc.allocations {
            println!("     [{:.0}%] {}", a.allocated_pct, a.name);
        }
    }
    {
        let mut engine = PortfolioEngine::new();
        engine
            .add_objective(PortfolioObjective::new(
                "Test objective",
                "desc",
                ObjectiveCategory::SecurityHardening,
                0.7,
                0.6,
                0.7,
                0.7,
                10.0,
            ))
            .expect("add");
        check(
            &mut tests,
            "Portfolio: objective count tracked correctly",
            engine.objective_count() == 1,
            "wrong",
        );
    }
    {
        let engine = PortfolioEngine::new();
        check(
            &mut tests,
            "Portfolio: empty portfolio returns error",
            engine.allocate(100.0, "Q1").is_err(),
            "no error",
        );
    }

    // ── HYDRA-CRYSTALLIZER ──
    println!("\n-- hydra-crystallizer --");
    {
        let mut src = CrystallizationSource::new("fintech")
            .with_approach("Rotate credentials 30min before window", 0.92)
            .with_approach("Use idempotency keys for settlement", 0.88)
            .with_approach("Pause and escalate for amounts above $10K", 0.95)
            .with_avoidable("concurrent lock on settlement ledger")
            .with_avoidable("missing credentials for settlement API");
        for i in 0..8 {
            src = src.with_success(srec(&format!("s{}", i), "fintech"));
        }
        for i in 0..2 {
            src = src.with_failure(frec(&format!("f{}", i), "fintech"));
        }

        let mut engine = CrystallizerEngine::new();
        let pb = engine.crystallize_playbook(&src).expect("playbook");
        check(
            &mut tests,
            "Crystallizer: playbook generated from 8 successful executions",
            pb.kind == ArtifactKind::Playbook,
            pb.kind.label(),
        );
        check(
            &mut tests,
            "Crystallizer: playbook contains header and pre-execution checklist",
            pb.content.contains("Playbook") && pb.content.contains("Checklist"),
            "missing",
        );
        check(
            &mut tests,
            "Crystallizer: confidence >= 0.60 (based on record count)",
            pb.confidence >= 0.60,
            &format!("{:.2}", pb.confidence),
        );
        println!("  info: {}", pb.summary_line());
        println!("  info: Playbook preview (first 3 lines):");
        for line in pb.content.lines().take(3) {
            println!("     {}", line);
        }
    }
    {
        let mut src = CrystallizationSource::new("fintech")
            .with_approach("Pre-provision credentials before deployment", 0.85)
            .with_avoidable("expired credentials at settlement time");
        for i in 0..2 {
            src = src.with_success(srec(&format!("s{}", i), "fintech"));
        }
        for i in 0..4 {
            src = src.with_failure(frec(&format!("f{}", i), "fintech"));
        }

        let mut engine = CrystallizerEngine::new();
        let pm = engine
            .crystallize_postmortem(&src, "Settlement auth failures — Q1 2026")
            .expect("postmortem");
        check(
            &mut tests,
            "Crystallizer: post-mortem generated from 4 failure records",
            pm.kind == ArtifactKind::PostMortem,
            pm.kind.label(),
        );
        check(
            &mut tests,
            "Crystallizer: post-mortem includes root causes section",
            pm.content.contains("Root Causes"),
            "section missing",
        );
    }
    {
        let src = CrystallizationSource::new("settlement")
            .with_approach("Idempotency keys prevent duplicate settlements", 0.95)
            .with_approach("Two-phase commit for large batches", 0.88)
            .with_approach("Settlement window lock prevents conflicts", 0.82);
        let mut engine = CrystallizerEngine::new();
        let kb = engine.crystallize_knowledge_base(&src).expect("kb");
        check(
            &mut tests,
            "Crystallizer: knowledge base generated from proven approaches",
            kb.kind == ArtifactKind::KnowledgeBase,
            kb.kind.label(),
        );
    }
    {
        let src = CrystallizationSource::new("new-domain");
        let mut engine = CrystallizerEngine::new();
        check(
            &mut tests,
            "Crystallizer: insufficient data -> error (not empty artifact)",
            engine.crystallize_playbook(&src).is_err(),
            "no error",
        );
    }

    // ── INTEGRATION ──
    println!("\n-- integration: settlement + attribution + portfolio + crystallizer --");
    {
        let mut settle = SettlementEngine::new();
        let mut attr = AttributionEngine::new();
        let mut portfolio = PortfolioEngine::new();
        let mut crystal = CrystallizerEngine::new();

        for _i in 0..7 {
            settle
                .settle_skill_action(
                    "agentra-settlement",
                    "settlement.execute",
                    "fintech",
                    "execute settlement batch",
                    1800,
                    3000,
                    true,
                )
                .expect("ok");
        }
        for _i in 0..3 {
            settle
                .settle_skill_action(
                    "agentra-settlement",
                    "settlement.execute",
                    "fintech",
                    "execute settlement with concurrent lock",
                    1200,
                    2000,
                    false,
                )
                .expect("ok");
        }
        for i in 0..7 {
            let _ = attr.attribute(&srec(&format!("a-s{}", i), "fintech"));
        }
        for i in 0..3 {
            let _ = attr.attribute(&frec(&format!("a-f{}", i), "fintech"));
        }

        let av = attr.avoidability_report(Some("fintech"));
        portfolio.ingest_avoidability(&av);
        portfolio
            .add_objective(PortfolioObjective::new(
                "Continue settlement execution",
                "Core business objective — ongoing",
                ObjectiveCategory::BusinessExecution,
                0.60,
                0.90,
                0.90,
                0.80,
                40.0,
            ))
            .expect("add business");
        let alloc = portfolio.allocate(100.0, "next-month").expect("allocate");

        let mut src = CrystallizationSource::new("fintech")
            .with_approach("Rotate credentials before settlement window", 0.92)
            .with_approach("Use idempotency keys", 0.88)
            .with_avoidable("concurrent lock on settlement");
        for i in 0..7 {
            src = src.with_success(srec(&format!("s{}", i), "fintech"));
        }
        for i in 0..3 {
            src = src.with_failure(frec(&format!("f{}", i), "fintech"));
        }
        let _ = crystal.crystallize_playbook(&src);

        check(
            &mut tests,
            "Integration: 10 settlement records -> 10 attribution trees",
            settle.record_count() == 10 && attr.tree_count() == 10,
            &format!(
                "settle={} attr={}",
                settle.record_count(),
                attr.tree_count()
            ),
        );
        check(
            &mut tests,
            "Integration: portfolio allocation generated from settlement data",
            !alloc.allocations.is_empty(),
            "no allocations",
        );
        check(
            &mut tests,
            "Integration: playbook crystallized from operational history",
            crystal.artifact_count() >= 1,
            "no artifacts",
        );
        println!("  info: Settle: {}", settle.summary());
        println!("  info: Attribution: {}", attr.summary());
        println!("  info: Portfolio: {}", alloc.brief());
        println!("  info: Crystallizer: {}", crystal.summary());
    }

    // ── RESULTS ──
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
    println!("\n=======================================================");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        std::process::exit(1);
    } else {
        println!();
        println!("  hydra-portfolio:    Resources allocated. Security ranked first.");
        println!("  hydra-crystallizer: Playbook from 8 real executions. Not templates.");
        println!("  Full Layer 5 pipeline: settlement -> attribution -> portfolio -> crystal.");
        println!("  Layer 5, Phases 3+4 complete.");
        println!("  Next: hydra-exchange — the final Layer 5 crate.");
        println!("=======================================================");
    }
}
