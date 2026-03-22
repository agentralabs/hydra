//! Phase 43 Test Harness — hydra-succession
//! Run: cargo run -p hydra-succession --bin test_harness

use hydra_succession::{
    InstanceState, SuccessionEngine, SuccessionError, SuccessionExporter, SuccessionVerifier,
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

fn v1_state(days: u32) -> InstanceState {
    InstanceState {
        instance_id: "hydra-v1".into(),
        lineage_id: "hydra-agentra-lineage".into(),
        days_running: days,
        soul_entries: days as usize / 10,
        genome_entries: days as usize * 2,
        calibration_profiles: 47,
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  Phase 43 — hydra-succession");
    println!("  Layer 7, Phase 1: Knowledge Transfer Across Generations");
    println!("  \"The entity survives the substrate change.\"");
    println!("═══════════════════════════════════════════════════════");

    let mut tests = Vec::new();

    // ── PACKAGE SEALING ───────────────────────────────────────────
    println!("\n── package sealing ──────────────────────────────────");

    {
        let exporter = SuccessionExporter::new();
        let state = v1_state(7300);
        let package = exporter.export(&state).expect("should export");

        if package.wisdom_days == 7300 {
            tests.push(Test::pass("Package: 7300 days of wisdom sealed (20 years)"));
        } else {
            tests.push(Test::fail(
                "Package: wisdom days",
                package.wisdom_days.to_string(),
            ));
        }

        if package.verify_integrity() && package.integrity_hash.len() == 64 {
            tests.push(Test::pass(
                "Package: SHA256 integrity hash sealed (64 chars)",
            ));
        } else {
            tests.push(Test::fail("Package: integrity", "hash invalid"));
        }

        if package.soul_entry_count() == 730 && package.genome_entry_count() == 14600 {
            tests.push(Test::pass(
                "Package: soul + genome counts match instance state",
            ));
        } else {
            tests.push(Test::fail(
                "Package: counts",
                format!(
                    "soul={} genome={}",
                    package.soul_entry_count(),
                    package.genome_entry_count()
                ),
            ));
        }

        if !package.is_expired() {
            tests.push(Test::pass("Package: not expired (valid for 7 days)"));
        } else {
            tests.push(Test::fail("Package: expiry", "already expired"));
        }

        println!(
            "  i  Package sealed: {} soul, {} genome, {} calibration, {} days",
            package.soul_entry_count(),
            package.genome_entry_count(),
            package.calibration_profile_count(),
            package.wisdom_days
        );
    }

    // ── THREE-GATE VERIFICATION ───────────────────────────────────
    println!("\n── three-gate verification ──────────────────────────");

    {
        let exporter = SuccessionExporter::new();
        let verifier = SuccessionVerifier::new();
        let package = exporter.export(&v1_state(7300)).expect("should export");

        let result = verifier
            .verify(&package, "hydra-agentra-lineage")
            .expect("should verify");

        if result.integrity_ok {
            tests.push(Test::pass("Verify Gate 1: integrity check passed"));
        } else {
            tests.push(Test::fail("Verify Gate 1", "integrity failed"));
        }

        if result.identity_ok {
            tests.push(Test::pass(
                "Verify Gate 2: identity check passed (lineage + morphic)",
            ));
        } else {
            tests.push(Test::fail("Verify Gate 2", "identity failed"));
        }

        if result.constitution_ok {
            tests.push(Test::pass(
                "Verify Gate 3: constitutional compliance confirmed",
            ));
        } else {
            tests.push(Test::fail("Verify Gate 3", "constitution failed"));
        }

        if result.all_pass() && result.notes.len() == 3 {
            tests.push(Test::pass("Verify: all 3 gates pass, 3 notes produced"));
        } else {
            tests.push(Test::fail(
                "Verify: gates",
                format!("notes={}", result.notes.len()),
            ));
        }

        for note in &result.notes {
            println!("  i  {}", note);
        }
    }

    // ── GATE FAILURES ─────────────────────────────────────────────
    println!("\n── gate failures ────────────────────────────────────");

    {
        let exporter = SuccessionExporter::new();
        let verifier = SuccessionVerifier::new();
        let package = exporter.export(&v1_state(7300)).expect("should export");
        let r = verifier.verify(&package, "different-lineage-id");
        if let Err(SuccessionError::IdentityMismatch) = r {
            tests.push(Test::pass(
                "Verify: wrong lineage -> IdentityMismatch (Gate 2)",
            ));
        } else {
            tests.push(Test::fail("Verify: wrong lineage", "wrong error"));
        }
    }

    {
        use hydra_succession::package::SuccessionPackage;
        use hydra_succession::payload::*;
        let package = SuccessionPackage::seal(
            "hydra-v1",
            "hydra-agentra-lineage",
            SoulPayload {
                entries: vec![],
                days_accumulated: 0,
                founding_statement: "".into(),
            },
            GenomePayload::simulated(10),
            CalibrationPayload::simulated(3),
            MorphicPayload::simulated(100, "hydra-agentra-lineage"),
        );
        let verifier = SuccessionVerifier::new();
        let r = verifier.verify(&package, "hydra-agentra-lineage");
        if let Err(SuccessionError::ConstitutionalViolation { .. }) = r {
            tests.push(Test::pass(
                "Verify: empty soul -> ConstitutionalViolation (Gate 3)",
            ));
        } else {
            tests.push(Test::fail("Verify: empty soul", "wrong error"));
        }
    }

    // ── FULL SUCCESSION ───────────────────────────────────────────
    println!("\n── full succession protocol ─────────────────────────");
    println!("  Scenario: Hydra v1 (7300 days) -> Hydra v2 (fresh boot)");

    {
        let v1 = v1_state(7300);
        let mut v2_engine = SuccessionEngine::new();
        let result = v2_engine.full_succession(&v1).expect("should succeed");

        if result.wisdom_days == 7300 {
            tests.push(Test::pass(
                "Succession: 7300 days of wisdom transferred to v2",
            ));
        } else {
            tests.push(Test::fail(
                "Succession: wisdom days",
                result.wisdom_days.to_string(),
            ));
        }

        if result.soul_entries == 730 {
            tests.push(Test::pass(
                "Succession: 730 soul entries transferred (20yr orientation)",
            ));
        } else {
            tests.push(Test::fail(
                "Succession: soul entries",
                result.soul_entries.to_string(),
            ));
        }

        if result.genome_entries == 14600 {
            tests.push(Test::pass(
                "Succession: 14600 genome entries transferred (20yr wisdom)",
            ));
        } else {
            tests.push(Test::fail(
                "Succession: genome entries",
                result.genome_entries.to_string(),
            ));
        }

        if v2_engine.has_imported() {
            tests.push(Test::pass("Succession: v2 is_imported = true"));
        } else {
            tests.push(Test::fail("Succession: imported flag", "false"));
        }

        println!("  i  {}", result.summary());
        for note in result.notes.iter().take(3) {
            println!("     {}", note);
        }
    }

    // ── DOUBLE IMPORT PROTECTION ──────────────────────────────────
    {
        let mut engine = SuccessionEngine::new();
        engine
            .full_succession(&v1_state(1000))
            .expect("first should succeed");
        let second = engine.full_succession(&v1_state(2000));
        if let Err(SuccessionError::AlreadyImported) = second {
            tests.push(Test::pass(
                "Protection: double import rejected (one-time per instance)",
            ));
        } else {
            tests.push(Test::fail(
                "Protection: double import",
                "allowed second import",
            ));
        }
    }

    // ── IDENTITY CONTINUITY ───────────────────────────────────────
    println!("\n── identity continuity ──────────────────────────────");
    println!("  \"The entity that booted on day 1 is the same entity on day 7300.\"");

    {
        let exporter = SuccessionExporter::new();
        let state = v1_state(7300);
        let package = exporter.export(&state).expect("should export");

        let identity_intact = package.lineage_id == "hydra-agentra-lineage"
            && package.morphic.lineage_id == "hydra-agentra-lineage"
            && package.morphic.days_depth == 7300
            && package.morphic.verify()
            && !package.morphic.signature_chain.is_empty();

        if identity_intact {
            tests.push(Test::pass(
                "Identity: lineage continuous. Morphic signature 7300 days deep. Chain intact.",
            ));
        } else {
            tests.push(Test::fail("Identity: continuity", "not intact"));
        }

        println!("  i  Lineage: {}", package.lineage_id);
        println!("  i  Morphic depth: {} days", package.morphic.days_depth);
        println!(
            "  i  Signature chain: {} checkpoints",
            package.morphic.signature_chain.len()
        );
    }

    // ── SUMMARY ───────────────────────────────────────────────────
    {
        let engine = SuccessionEngine::new();
        let s = engine.summary();
        if s.contains("succession:") && s.contains("exported=") && s.contains("imported=") {
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
        println!("  hydra-succession verified.");
        println!("  Package sealing:    SHA256 signed, 7-day validity.");
        println!("  Three gates:        integrity -> identity -> constitution.");
        println!("  Full succession:    20yr wisdom transferred in one protocol.");
        println!("  Double import:      one-time per instance, enforced.");
        println!("  Identity:           lineage continuous, morphic intact.");
        println!("  Layer 7, Phase 1 complete.");
        println!("  Next: hydra-legacy + hydra-continuity (parallel).");
        println!("═══════════════════════════════════════════════════════");
    }
}
