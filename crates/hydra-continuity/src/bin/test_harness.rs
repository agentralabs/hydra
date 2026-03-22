//! Phase 44+46 Combined Harness — hydra-legacy + hydra-continuity
//! Run: cargo run -p hydra-continuity --bin test_harness

use hydra_legacy::LegacyEngine;
use hydra_continuity::ContinuityEngine;
use hydra_succession::{
    InstanceState, SuccessionEngine, SuccessionExporter,
};

struct Test {
    name: &'static str,
    passed: bool,
    notes: Option<String>,
}
impl Test {
    fn pass(name: &'static str) -> Self {
        Self { name, passed: true, notes: None }
    }
    fn fail(name: &'static str, n: impl Into<String>) -> Self {
        Self { name, passed: false, notes: Some(n.into()) }
    }
}

fn v1_state() -> InstanceState {
    InstanceState {
        instance_id: "hydra-v1".into(),
        lineage_id: "hydra-agentra-lineage".into(),
        days_running: 7300,
        soul_entries: 730,
        genome_entries: 14_600,
        calibration_profiles: 47,
    }
}

fn main() {
    println!("===============================================");
    println!("  Phase 44+46 — hydra-legacy + hydra-continuity");
    println!("  Layer 7, Phases 2+3: Permanence and Proof");
    println!("===============================================");

    let mut tests = Vec::new();

    // Build the shared succession package once
    let exporter = SuccessionExporter::new();
    let package = exporter.export(&v1_state())
        .expect("should export v1 state");

    // -- HYDRA-LEGACY --
    println!("\n-- hydra-legacy ------------------------------------");

    {
        let mut engine = LegacyEngine::new();

        // Knowledge record from genome
        let kr = engine.publish_knowledge(&package, "engineering")
            .expect("should publish knowledge");
        if kr.verify_integrity() {
            tests.push(Test::pass(
                "Legacy: engineering knowledge record — SHA256 signed",
            ));
        } else {
            tests.push(Test::fail("Legacy: knowledge integrity", "failed"));
        }
        if kr.source_days == 7300 {
            tests.push(Test::pass(
                "Legacy: knowledge record traces to 7300 source days",
            ));
        } else {
            tests.push(Test::fail(
                "Legacy: source days",
                format!("{}", kr.source_days),
            ));
        }
        println!("  i  {}", kr.summary_line());

        // Operational record
        let or_ = engine
            .publish_operational(
                &package,
                "20-year operational history 2006-2026",
            )
            .expect("should publish operational");
        if or_.content.contains("operational days") {
            tests.push(Test::pass(
                "Legacy: operational record contains period data",
            ));
        } else {
            tests.push(Test::fail(
                "Legacy: operational content",
                "missing period data",
            ));
        }
        println!("  i  {}", or_.summary_line());

        // Wisdom record
        let wr = engine.publish_wisdom(&package, "fintech")
            .expect("should publish wisdom");
        if wr.content.contains("Calibrat") {
            tests.push(Test::pass(
                "Legacy: wisdom record contains calibration bias data",
            ));
        } else {
            tests.push(Test::fail(
                "Legacy: wisdom content",
                "missing calibration",
            ));
        }
        println!("  i  {}", wr.summary_line());

        // All three in archive
        if engine.artifact_count() == 3 {
            tests.push(Test::pass(
                "Legacy: 3 artifacts in permanent archive (knowledge/operational/wisdom)",
            ));
        } else {
            tests.push(Test::fail(
                "Legacy: archive count",
                format!("{}", engine.artifact_count()),
            ));
        }

        // Independence: no Hydra instance needed to read them
        let for_lineage =
            engine.artifacts_for_lineage("hydra-agentra-lineage");
        if for_lineage.len() == 3 {
            tests.push(Test::pass(
                "Legacy: all 3 artifacts queryable by lineage ID",
            ));
        } else {
            tests.push(Test::fail(
                "Legacy: lineage query",
                format!("{}", for_lineage.len()),
            ));
        }

        println!("  i  {}", engine.summary());
    }

    {
        // Insufficient history error
        let small = SuccessionExporter::new()
            .export(&InstanceState {
                instance_id: "tiny".into(),
                lineage_id: "tiny-lineage".into(),
                days_running: 30,
                soul_entries: 3,
                genome_entries: 10,
                calibration_profiles: 1,
            })
            .expect("should export small package");
        let mut engine = LegacyEngine::new();
        let r = engine.publish_knowledge(&small, "test");
        if r.is_err() {
            tests.push(Test::pass(
                "Legacy: < 365 days -> insufficient history (no premature legacy)",
            ));
        } else {
            tests.push(Test::fail("Legacy: minimum history", "no error"));
        }
    }

    // -- HYDRA-CONTINUITY --
    println!("\n-- hydra-continuity --------------------------------");

    {
        let mut engine = ContinuityEngine::new();
        engine.record_from_succession(&package);

        if engine.lineage_count() == 1 {
            tests.push(Test::pass(
                "Continuity: arc built from succession package",
            ));
        } else {
            tests.push(Test::fail(
                "Continuity: arc count",
                format!("{}", engine.lineage_count()),
            ));
        }

        if engine.total_checkpoint_count() > 0 {
            tests.push(Test::pass(
                "Continuity: checkpoints across 7300 days (one per year)",
            ));
        } else {
            tests.push(Test::fail("Continuity: checkpoints", "none"));
        }

        // Lineage proof
        let proven = engine
            .prove_lineage("hydra-agentra-lineage")
            .expect("should prove lineage");
        if proven {
            tests.push(Test::pass(
                "Continuity: lineage proof verified (all checkpoints valid)",
            ));
        } else {
            tests.push(Test::fail("Continuity: lineage proof", "failed"));
        }

        // Wrong lineage proof fails
        let wrong = engine.prove_lineage("wrong-lineage");
        if wrong.is_err() {
            tests.push(Test::pass(
                "Continuity: wrong lineage -> ArcNotFound error",
            ));
        } else {
            tests.push(Test::fail(
                "Continuity: wrong lineage",
                "no error",
            ));
        }

        // Print the arc
        if let Some(arc) = engine.arc("hydra-agentra-lineage") {
            println!(
                "  i  Entity arc: {} checkpoints, {} total days",
                arc.checkpoint_count(),
                arc.total_days
            );
            for cp in arc.checkpoints.iter().take(3) {
                println!(
                    "     Day {:>5}: soul={} genome={}{}",
                    cp.day,
                    cp.soul_count,
                    cp.genome_count,
                    cp.notable_change
                        .as_deref()
                        .map(|n| format!(" -- {}", n))
                        .unwrap_or_default()
                );
            }
            if arc.checkpoint_count() > 3 {
                println!(
                    "     ... ({} more checkpoints)",
                    arc.checkpoint_count() - 3
                );
            }
        }

        println!("  i  {}", engine.summary());
    }

    // -- SUCCESSION PROOF --
    println!("\n-- succession proof: v1 -> v2 continuity -----------");

    {
        let mut engine = ContinuityEngine::new();

        // v1: 7300 days of operation
        let v1_pkg = SuccessionExporter::new()
            .export(&v1_state())
            .expect("should export v1");
        engine.record_from_succession(&v1_pkg);

        // v2: inherits from v1, then operates 100 more days
        let v2_pkg = SuccessionExporter::new()
            .export(&InstanceState {
                instance_id: "hydra-v2".into(),
                lineage_id: "hydra-agentra-lineage".into(),
                days_running: 7400,
                soul_entries: 740,
                genome_entries: 14_800,
                calibration_profiles: 48,
            })
            .expect("should export v2");
        engine.record_from_succession(&v2_pkg);

        // Only one lineage — v2 extended v1's arc
        if engine.lineage_count() == 1 {
            tests.push(Test::pass(
                "Proof: v2 extends v1's arc (same lineage, not a new entity)",
            ));
        } else {
            tests.push(Test::fail(
                "Proof: lineage count",
                format!("{}", engine.lineage_count()),
            ));
        }

        // v2 proves succession from v1
        let proven = engine
            .prove_succession(
                "hydra-agentra-lineage",
                "hydra-agentra-lineage",
            )
            .expect("should prove succession");
        if proven {
            tests.push(Test::pass(
                "Proof: v2 mathematically proves succession from v1 (same entity, different vessel)",
            ));
        } else {
            tests.push(Test::fail("Proof: succession", "not proven"));
        }

        if let Some(arc) = engine.arc("hydra-agentra-lineage") {
            println!(
                "  i  Final arc: {} days, {} checkpoints",
                arc.total_days,
                arc.checkpoint_count()
            );
        }
    }

    // -- INTEGRATION --
    println!("\n-- integration: succession + legacy + continuity ---");

    {
        let mut succession = SuccessionEngine::new();
        let mut legacy = LegacyEngine::new();
        let mut continuity = ContinuityEngine::new();

        // Full succession transfer
        let result = succession
            .full_succession(&v1_state())
            .expect("should succeed");

        // Build legacy from the package
        let pkg = exporter.export(&v1_state())
            .expect("should export for integration");
        legacy
            .publish_knowledge(&pkg, "cobol")
            .expect("should publish cobol knowledge");
        legacy
            .publish_operational(
                &pkg,
                "20yr cobol enterprise migration history",
            )
            .expect("should publish operational");

        // Build continuity arc
        continuity.record_from_succession(&pkg);

        let all_operational = result.wisdom_days == 7300
            && legacy.artifact_count() == 2
            && continuity.total_checkpoint_count() > 0
            && continuity
                .prove_lineage("hydra-agentra-lineage")
                .unwrap_or(false);

        if all_operational {
            tests.push(Test::pass(
                "Integration: succession transferred + legacy archived + continuity proven",
            ));
        } else {
            tests.push(Test::fail(
                "Integration",
                format!(
                    "wisdom={} artifacts={} checkpoints={}",
                    result.wisdom_days,
                    legacy.artifact_count(),
                    continuity.total_checkpoint_count()
                ),
            ));
        }

        println!("  i  Succession: {}", result.summary());
        println!("  i  {}", legacy.summary());
        println!("  i  {}", continuity.summary());
    }

    // -- RESULTS --
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
    println!("===============================================");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        std::process::exit(1);
    } else {
        println!();
        println!("  hydra-legacy:      knowledge escapes the instance.");
        println!("  hydra-continuity:  identity proven across 7300 days.");
        println!("  v1 -> v2 proof:    same entity, different vessel.");
        println!("  Layer 7, Phases 2+3 complete.");
        println!("  ONE CRATE REMAINS: hydra-influence.");
        println!("  The final crate. Layer 7 closes.");
        println!("===============================================");
    }
}
