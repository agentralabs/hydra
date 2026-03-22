//! Phase 28 Test Harness — hydra-omniscience
//! Run: cargo run -p hydra-omniscience --bin test_harness

use hydra_omniscience::{
    AcquisitionSource, GapType, OmniscienceEngine,
    OmniscienceError, AcquisitionResult,
};
use hydra_omniscience::constants::{
    RECURRING_GAP_THRESHOLD, MIN_ACQUISITION_CONFIDENCE,
};

struct Test { name: &'static str, passed: bool, notes: Option<String> }
impl Test {
    fn pass(name: &'static str) -> Self {
        Self { name, passed: true, notes: None }
    }
    fn fail(name: &'static str, n: impl Into<String>) -> Self {
        Self { name, passed: false, notes: Some(n.into()) }
    }
}

fn main() {
    println!("=======================================================");
    println!("  Phase 28 -- hydra-omniscience");
    println!("  Layer 4, Phase 1: Active Knowledge Acquisition");
    println!("  \"I don't know yet. Acquiring.\"");
    println!("=======================================================");

    let mut tests  = Vec::new();
    let mut engine = OmniscienceEngine::new();

    // -- GAP DETECTION ------------------------------------------------
    println!("\n-- gap detection ----------------------------------------");

    {
        let id = engine.detect_gap(
            "kubernetes rolling update maxSurge parameter",
            GapType::Procedural { action: "rolling-update".into() },
            0.85,
        );
        if engine.gap_count() == 1 && !id.is_empty() {
            tests.push(Test::pass("Gap: detected and registered with ID"));
        } else {
            tests.push(Test::fail(
                "Gap: detection",
                format!("count={}", engine.gap_count()),
            ));
        }
        if engine.open_gaps().len() == 1 {
            tests.push(Test::pass("Gap: appears in open_gaps()"));
        } else {
            tests.push(Test::fail(
                "Gap: open gaps",
                format!("{}", engine.open_gaps().len()),
            ));
        }
    }

    {
        // Same topic twice -> recurrence increments, not duplicate gap
        engine.detect_gap(
            "kubernetes rolling update maxSurge parameter",
            GapType::Procedural { action: "rolling-update".into() },
            0.85,
        );
        if engine.gap_count() == 1 {
            tests.push(Test::pass(
                "Gap: same topic -> recurrence incremented (no duplicate)",
            ));
        } else {
            tests.push(Test::fail(
                "Gap: dedup",
                format!("{} gaps", engine.gap_count()),
            ));
        }
    }

    // -- SOURCE RELIABILITY -------------------------------------------
    println!("\n-- source reliability -----------------------------------");

    {
        let cb  = AcquisitionSource::AgenticCodebase { query: "test".into() };
        let syn = AcquisitionSource::BeliefSynthesis {
            related_topics: vec![],
        };
        let web = AcquisitionSource::WebSearch { query: "test".into() };
        if cb.reliability() > syn.reliability()
            && syn.reliability() > web.reliability()
        {
            tests.push(Test::pass(
                "Sources: codebase > synthesis > web (reliability ordering)",
            ));
        } else {
            tests.push(Test::fail(
                "Sources: reliability order",
                format!(
                    "{:.2} {:.2} {:.2}",
                    cb.reliability(), syn.reliability(), web.reliability()
                ),
            ));
        }
    }

    {
        let sources = AcquisitionSource::for_gap(
            "how to implement circuit breaker", "engineering",
        );
        if sources[0].label() == "agentic-codebase" {
            tests.push(Test::pass(
                "Sources: engineering gap -> codebase first",
            ));
        } else {
            tests.push(Test::fail(
                "Sources: engineering first",
                sources[0].label().to_string(),
            ));
        }
    }

    // -- ACQUISITION --------------------------------------------------
    println!("\n-- acquisition -----------------------------------------");

    {
        let result = engine.detect_and_acquire(
            "circuit breaker pattern in distributed systems",
            GapType::Structural { relationship: "circuit-breaker".into() },
            0.9,
        );

        match result {
            Ok(ref r) => {
                if r.closed && r.confidence >= MIN_ACQUISITION_CONFIDENCE {
                    tests.push(Test::pass(
                        "Acquire: gap closed with sufficient confidence",
                    ));
                } else {
                    tests.push(Test::fail(
                        "Acquire: closure",
                        format!(
                            "closed={} conf={:.2}", r.closed, r.confidence
                        ),
                    ));
                }

                if engine.closed_count() == 1 {
                    tests.push(Test::pass("Acquire: closed_count = 1"));
                } else {
                    tests.push(Test::fail(
                        "Acquire: closed count",
                        format!("{}", engine.closed_count()),
                    ));
                }

                if engine.open_gaps().len() == 1 {
                    // k8s gap still open, circuit breaker closed
                    tests.push(Test::pass(
                        "Acquire: closed gap removed from open_gaps",
                    ));
                } else {
                    tests.push(Test::fail(
                        "Acquire: open gaps after close",
                        format!("{}", engine.open_gaps().len()),
                    ));
                }

                println!("  i  Acquisition result:");
                println!("     topic:      {}", r.topic);
                println!("     source:     {}", r.source);
                println!("     confidence: {:.2}", r.confidence);
            }
            Err(e) => {
                tests.push(Test::fail("Acquire: closure", format!("{}", e)));
                tests.push(Test::fail("Acquire: closed count", "err"));
                tests.push(Test::fail("Acquire: open gaps after close", "err"));
            }
        }
    }

    // -- RECURRING GAPS -----------------------------------------------
    println!("\n-- recurring gaps --------------------------------------");

    {
        for _ in 0..RECURRING_GAP_THRESHOLD {
            engine.detect_gap(
                "video editing codec vocabulary",
                GapType::VocabularyMissing { domain: "video".into() },
                0.6,
            );
        }

        if !engine.recurring_gaps().is_empty() {
            tests.push(Test::pass(
                "Recurring: gap flagged as recurring (skill load signal)",
            ));
            println!(
                "  i  Recurring gap: 'video editing codec vocabulary' \
                 -- signal: load video-editor skill"
            );
        } else {
            tests.push(Test::fail("Recurring: flag", "not flagged"));
        }
    }

    // -- BELIEF INTEGRATION -------------------------------------------
    println!("\n-- belief integration -----------------------------------");

    {
        let result = AcquisitionResult::new(
            "gap-test",
            "agentic-codebase",
            "Kubernetes maxSurge controls how many extra pods can exist \
             during update. Default is 25%. MaxUnavailable controls how \
             many pods can be unavailable.",
            0.88,
            "github.com/kubernetes/kubernetes/blob/main/pkg/apis/apps/v1/types.go",
        );

        let belief_stmt = result.belief_statement();
        if belief_stmt.contains("agentic-codebase")
            && belief_stmt.contains("0.88")
        {
            tests.push(Test::pass(
                "Belief: statement contains source and confidence for manifold",
            ));
        } else {
            let end = belief_stmt.len().min(80);
            tests.push(Test::fail(
                "Belief: statement",
                belief_stmt[..end].to_string(),
            ));
        }

        if result.meets_threshold() {
            tests.push(Test::pass(
                "Belief: high-confidence result enters manifold",
            ));
        } else {
            tests.push(Test::fail(
                "Belief: threshold",
                format!("{:.2}", result.confidence),
            ));
        }
    }

    // -- FULL PIPELINE ------------------------------------------------
    println!("\n-- full pipeline: detect -> plan -> acquire -> close ----");

    {
        let mut fresh = OmniscienceEngine::new();

        let topics = vec![
            (
                "GraphQL subscription protocol",
                GapType::ApiSpec { service: "graphql".into() },
                0.75,
            ),
            (
                "PostgreSQL JSONB indexing",
                GapType::Procedural { action: "jsonb-index".into() },
                0.80,
            ),
            (
                "AWS Lambda cold start optimization",
                GapType::Procedural { action: "cold-start".into() },
                0.70,
            ),
        ];

        let mut all_closed = true;
        for (topic, gap_type, priority) in topics {
            match fresh.detect_and_acquire(topic, gap_type, priority) {
                Ok(result) => {
                    if !result.closed { all_closed = false; }
                }
                Err(_) => { all_closed = false; }
            }
        }

        if all_closed && fresh.closed_count() == 3 {
            tests.push(Test::pass(
                "Pipeline: 3 gaps detected, planned, acquired, and closed",
            ));
        } else {
            tests.push(Test::fail(
                "Pipeline",
                format!(
                    "closed={} all_closed={}",
                    fresh.closed_count(), all_closed
                ),
            ));
        }

        println!("  i  {}", fresh.summary());
    }

    // -- ERROR HANDLING -----------------------------------------------
    {
        let err = OmniscienceError::GapUnresolvable {
            topic: "impossible topic".into(),
        };
        if err.requires_human() {
            tests.push(Test::pass(
                "Error: GapUnresolvable correctly flags for human escalation",
            ));
        } else {
            tests.push(Test::fail("Error: human flag", "not flagged"));
        }
    }

    // -- SUMMARY ------------------------------------------------------
    {
        let s = engine.summary();
        if s.contains("omniscience:")
            && s.contains("gaps=")
            && s.contains("closed=")
        {
            tests.push(Test::pass(
                "Summary: format correct for TUI display",
            ));
        } else {
            tests.push(Test::fail("Summary", s.to_string()));
        }
        println!("\n  i  {}", engine.summary());
    }

    // -- RESULTS ------------------------------------------------------
    println!();
    let total  = tests.len();
    let passed = tests.iter().filter(|t| t.passed).count();
    let failed = total - passed;

    for t in &tests {
        if t.passed {
            println!("  PASS  {}", t.name);
        } else {
            println!("  FAIL  {}", t.name);
            if let Some(n) = &t.notes { println!("           {}", n); }
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
        println!("  hydra-omniscience verified.");
        println!("  Gap detection:      operational.");
        println!("  Acquisition plans:  built per gap type.");
        println!("  Source hierarchy:   codebase -> docs -> synthesis -> web.");
        println!("  Belief integration: confidence-gated.");
        println!("  Recurring gaps:     flagged for skill loading.");
        println!("  Layer 4, Phase 1 complete.");
        println!("  Next: hydra-pattern + hydra-redteam (parallel).");
        println!("=======================================================");
    }
}
