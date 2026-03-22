//! Phase 39+41 Combined Harness — hydra-consensus + hydra-consent
//! Run: cargo run -p hydra-consent --bin test_harness

mod consensus_tests;
mod consent_tests;
mod integration_tests;

pub struct Test {
    name: &'static str,
    passed: bool,
    notes: Option<String>,
}
impl Test {
    pub fn pass(name: &'static str) -> Self {
        Self {
            name,
            passed: true,
            notes: None,
        }
    }
    pub fn fail(name: &'static str, n: impl Into<String>) -> Self {
        Self {
            name,
            passed: false,
            notes: Some(n.into()),
        }
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  Phase 39+41 — hydra-consensus + hydra-consent");
    println!("  Layer 6, Phases 2+3: Resolution and Governance");
    println!("═══════════════════════════════════════════════════════");

    let mut tests = Vec::new();

    // ── HYDRA-CONSENSUS ───────────────────────────────────────────
    consensus_tests::run(&mut tests);

    // ── HYDRA-CONSENT ─────────────────────────────────────────────
    consent_tests::run(&mut tests);

    // ── INTEGRATION: FEDERATION + CONSENSUS + CONSENT ─────────────
    integration_tests::run(&mut tests);

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
        std::process::exit(1);
    } else {
        println!();
        println!("  hydra-consensus: OK  Neither belief simply overwrites.");
        println!("  hydra-consent:   OK  No consent -> no sharing. Hard stop.");
        println!("  Integration:     OK  federation + consent + consensus chained.");
        println!("  Layer 6, Phases 2+3 complete.");
        println!("  Next: hydra-collective — distributed pattern intelligence.");
        println!("═══════════════════════════════════════════════════════");
    }
}
