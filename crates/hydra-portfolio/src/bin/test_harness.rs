//! Phase 35 — hydra-portfolio test harness
//! Run: cargo run -p hydra-portfolio --bin test_harness

use hydra_portfolio::{
    constants::DEFAULT_ATTENTION_BUDGET, ObjectiveCategory, PortfolioEngine, PortfolioError,
    PortfolioObjective,
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
    println!("===================================================");
    println!("  Phase 35 — hydra-portfolio");
    println!("  Layer 5, Phase 3: Strategic Resource Allocation");
    println!("===================================================");

    let mut tests = Vec::new();

    // ── Objective creation and scoring ──
    println!("\n-- objective creation and scoring --");

    {
        let mut engine = PortfolioEngine::new();
        engine
            .add_objective(PortfolioObjective::new(
                "Harden settlement auth surface",
                "Red team identified HIGH threat in settlement auth",
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
                "Attribution shows 28% of deployment cost is avoidable",
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
                "5 recurring knowledge gaps about supply domain detected",
                ObjectiveCategory::CapabilityExpansion,
                0.40,
                0.70,
                0.70,
                0.50,
                15.0,
            ))
            .expect("add capability");

        let alloc = engine
            .allocate(DEFAULT_ATTENTION_BUDGET, "Q2-2026")
            .expect("allocate");

        if !alloc.allocations.is_empty() {
            tests.push(Test::pass("3 objectives ranked and allocated"));
        } else {
            tests.push(Test::fail("allocations", "empty"));
        }

        // Security should rank highest (0.88 risk + 0.92 urgency)
        let top = alloc.top_recommendation().expect("top");
        if top.name.contains("settlement auth") {
            tests.push(Test::pass(
                "security objective ranks first (high risk + urgency)",
            ));
        } else {
            tests.push(Test::fail("top objective", top.name.to_string()));
        }

        // Allocations should sum to 100%
        let total_pct: f64 = alloc.allocations.iter().map(|a| a.allocated_pct).sum();
        if (total_pct - 100.0).abs() < 0.1 {
            tests.push(Test::pass("allocations sum to 100%"));
        } else {
            tests.push(Test::fail("sum", format!("{:.2}", total_pct)));
        }

        println!("  info: {}", alloc.brief());
        for a in &alloc.allocations {
            println!("     [{:.0}%] {}", a.allocated_pct, a.name);
        }
    }

    // ── Objective count ──
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
        let count = engine.objective_count();
        if count == 1 {
            tests.push(Test::pass("objective count tracked correctly"));
        } else {
            tests.push(Test::fail("count", format!("{}", count)));
        }
    }

    // ── Empty portfolio error ──
    {
        let engine = PortfolioEngine::new();
        let result = engine.allocate(100.0, "Q1");
        if matches!(result, Err(PortfolioError::NoObjectives)) {
            tests.push(Test::pass("empty portfolio returns error"));
        } else {
            tests.push(Test::fail("empty error", "no error"));
        }
    }

    // ── Results ──
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
    println!("===================================================");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        std::process::exit(1);
    } else {
        println!("  hydra-portfolio: Resources allocated. Security ranked first.");
        println!("===================================================");
    }
}
