//! Test harness for hydra-oracle.

use hydra_axiom::AxiomPrimitive;
use hydra_oracle::OracleEngine;

fn main() {
    println!("=== hydra-oracle test harness ===\n");

    let mut engine = OracleEngine::new();

    // Test 1: risk projection
    print!("  [1] Risk projection ... ");
    let proj = engine
        .project("deploy auth", "security", &[AxiomPrimitive::Risk])
        .expect("should project");
    assert!(proj.adverse_count() > 0, "should have adverse scenarios");
    println!("PASS");

    // Test 2: optimization projection
    print!("  [2] Optimization projection ... ");
    let proj = engine
        .project(
            "cache tuning",
            "performance",
            &[AxiomPrimitive::Optimization],
        )
        .expect("should project");
    assert_eq!(proj.adverse_count(), 0, "optimization should be positive");
    println!("PASS");

    // Test 3: cascade scenario
    print!("  [3] Cascade scenario ... ");
    let proj = engine
        .project(
            "service mesh",
            "infrastructure",
            &[AxiomPrimitive::CausalLink],
        )
        .expect("should project");
    assert!(proj.adverse_count() > 0, "cascade should be adverse");
    println!("PASS");

    // Test 4: scenario count
    print!("  [4] Scenario count ... ");
    assert!(
        engine.scenario_count() >= 3,
        "should have at least 3 scenarios"
    );
    println!("PASS (total={})", engine.scenario_count());

    // Test 5: summary
    print!("  [5] Summary ... ");
    let summary = engine.summary();
    assert!(
        summary.contains("OracleEngine"),
        "summary should have header"
    );
    println!("PASS");

    // Test 6: projection summary
    print!("  [6] Projection summary ... ");
    let proj = engine
        .project(
            "full deploy",
            "production",
            &[
                AxiomPrimitive::Risk,
                AxiomPrimitive::CausalLink,
                AxiomPrimitive::Optimization,
            ],
        )
        .expect("should project");
    let ps = proj.summary();
    assert!(
        ps.contains("full deploy"),
        "projection summary should have context"
    );
    println!("PASS");

    println!("\n  Oracle summary: {}", engine.summary());
    println!("\n=== hydra-oracle: ALL TESTS PASSED ===");
}
