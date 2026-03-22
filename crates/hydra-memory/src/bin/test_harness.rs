//! Test harness for hydra-memory.
//! Tests everything that doesn't require a live AgenticMemory file.
//! Bridge integration is tested in tests/memory.rs.
//!
//! Run: cargo run -p hydra-memory --bin test_harness

use hydra_memory::{
    constants::*,
    identity::IdentityProfile,
    layers::{MemoryLayer, MemoryRecord},
    session::{SessionManager, SessionRecord},
    temporal_bridge::TemporalBridge,
    verbatim::{ContextSnapshot, Surface, VerbatimRecord},
};
use hydra_temporal::{btree::ManifoldCoord, timestamp::Timestamp};

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

fn ts(n: u64) -> Timestamp {
    Timestamp::from_nanos(n).expect("valid nanos")
}

fn main() {
    println!("===================================================");
    println!("  hydra-memory -- Soul Test Harness");
    println!("===================================================");

    let mut tests = Vec::new();

    // -- Memory layers ---------------------------------------------------
    {
        if MemoryLayer::Verbatim.is_immutable() {
            tests.push(Test::pass("Layers: Verbatim is immutable"));
        } else {
            tests.push(Test::fail(
                "Layers: Verbatim is immutable",
                "should be immutable",
            ));
        }

        if MemoryLayer::Semantic.is_revisable() {
            tests.push(Test::pass("Layers: Semantic is revisable"));
        } else {
            tests.push(Test::fail(
                "Layers: Semantic is revisable",
                "should be revisable",
            ));
        }

        // All 8 layers have non-empty tags
        let layers = [
            MemoryLayer::Verbatim,
            MemoryLayer::Episodic,
            MemoryLayer::Semantic,
            MemoryLayer::Relational,
            MemoryLayer::Causal,
            MemoryLayer::Procedural,
            MemoryLayer::Anticipatory,
            MemoryLayer::Identity,
        ];
        let all_tagged = layers.iter().all(|l| !l.tag().is_empty());
        if all_tagged {
            tests.push(Test::pass("Layers: all 8 layers have non-empty tags"));
        } else {
            tests.push(Test::fail(
                "Layers: all 8 tags non-empty",
                "empty tag found",
            ));
        }
    }

    // -- Verbatim record -------------------------------------------------
    {
        let record = VerbatimRecord::begin(
            "session-001",
            0,
            Surface::Tui,
            "build the kernel",
            ContextSnapshot::default(),
            "const-identity",
        )
        .expect("should create record");

        if record.principal_input == "build the kernel" {
            tests.push(Test::pass("Verbatim: input stored exactly"));
        } else {
            tests.push(Test::fail("Verbatim: exact input", "mismatch"));
        }

        if record.hydra_response.is_none() {
            tests.push(Test::pass("Verbatim: response is None before finalization"));
        } else {
            tests.push(Test::fail(
                "Verbatim: response before finalize",
                "should be None",
            ));
        }

        // Finalize with SHA256
        let mut r = record;
        r.finalize("Phase 3 complete -- Hydra is breathing.", 0.05);

        if r.content_hash.is_some() {
            tests.push(Test::pass("Verbatim: SHA256 hash computed on finalization"));
        } else {
            tests.push(Test::fail("Verbatim: SHA256 hash", "not computed"));
        }

        // Integrity check passes
        if r.verify_integrity().is_ok() {
            tests.push(Test::pass(
                "Verbatim: integrity check passes on valid record",
            ));
        } else {
            tests.push(Test::fail(
                "Verbatim: integrity check",
                "failed on valid record",
            ));
        }

        // Tamper detection
        r.hydra_response = Some("tampered".to_string());
        if r.verify_integrity().is_err() {
            tests.push(Test::pass("Verbatim: tampered response detected by hash"));
        } else {
            tests.push(Test::fail("Verbatim: tamper detection", "not detected"));
        }
    }

    // -- Oversized input rejected ----------------------------------------
    {
        let big = "x".repeat(MAX_VERBATIM_SIZE_BYTES + 1);
        if VerbatimRecord::begin("s", 0, Surface::Tui, big, ContextSnapshot::default(), "r")
            .is_err()
        {
            tests.push(Test::pass("Verbatim: oversized input rejected"));
        } else {
            tests.push(Test::fail("Verbatim: oversized rejected", "should fail"));
        }
    }

    // -- Session management ----------------------------------------------
    {
        let s = SessionRecord::new();
        if !s.is_closed {
            tests.push(Test::pass("Session: new session starts open"));
        } else {
            tests.push(Test::fail("Session: starts open", "should be open"));
        }

        let mut s = SessionRecord::new();
        s.record_exchange();
        s.record_exchange();
        if s.exchange_count == 2 {
            tests.push(Test::pass("Session: exchange count tracked"));
        } else {
            tests.push(Test::fail(
                "Session: exchange count",
                format!("got {}", s.exchange_count),
            ));
        }

        let mut mgr = SessionManager::new();
        let id = mgr.session_id().to_string();
        for _ in 0..5 {
            mgr.record_exchange();
        }
        if mgr.session_id() == id {
            tests.push(Test::pass(
                "Session: manager maintains session across exchanges",
            ));
        } else {
            tests.push(Test::fail(
                "Session: stable session id",
                "changed unexpectedly",
            ));
        }
    }

    // -- Identity memory -------------------------------------------------
    {
        let mut p = IdentityProfile::new();

        if !p.is_confident {
            tests.push(Test::pass("Identity: new profile not confident"));
        } else {
            tests.push(Test::fail(
                "Identity: not confident initially",
                "should be false",
            ));
        }

        for _ in 0..IDENTITY_MIN_SESSIONS_FOR_CONFIDENCE {
            p.observe_session(45.0, 9);
        }

        if p.is_confident {
            tests.push(Test::pass(
                "Identity: becomes confident after enough sessions",
            ));
        } else {
            tests.push(Test::fail(
                "Identity: confidence",
                "not confident after min sessions",
            ));
        }

        p.observe("surface", "TUI", 0.9);
        p.observe("surface", "TUI", 0.95);
        if p.observations.len() == 1 && p.observations[0].observation_count == 2 {
            tests.push(Test::pass("Identity: repeated observation updates count"));
        } else {
            tests.push(Test::fail("Identity: observation update", "wrong count"));
        }
    }

    // -- Temporal bridge -------------------------------------------------
    {
        let mut bridge = TemporalBridge::new();

        bridge
            .index(
                "mem-001",
                ts(1_000_000_000),
                ManifoldCoord::new(0.1, 0.1, 0.0),
                "const-identity",
                "s1",
            )
            .expect("should index");
        bridge
            .index(
                "mem-002",
                ts(2_000_000_000),
                ManifoldCoord::new(0.2, 0.2, 0.0),
                "const-identity",
                "s1",
            )
            .expect("should index");
        bridge
            .index(
                "mem-003",
                ts(3_000_000_000),
                ManifoldCoord::new(0.3, 0.3, 0.0),
                "decision-xyz",
                "s1",
            )
            .expect("should index");

        if bridge.total_indexed() == 3 {
            tests.push(Test::pass("TemporalBridge: 3 memories indexed"));
        } else {
            tests.push(Test::fail(
                "TemporalBridge: total indexed",
                format!("got {}", bridge.total_indexed()),
            ));
        }

        if bridge.get_exact(&ts(2_000_000_000)).is_some() {
            tests.push(Test::pass("TemporalBridge: exact timestamp lookup"));
        } else {
            tests.push(Test::fail("TemporalBridge: exact lookup", "not found"));
        }

        let recent = bridge.most_recent(2);
        if recent.len() == 2 {
            tests.push(Test::pass("TemporalBridge: most_recent(2) returns 2"));
        } else {
            tests.push(Test::fail(
                "TemporalBridge: most_recent",
                format!("got {}", recent.len()),
            ));
        }

        let causal = bridge.by_causal_root("const-identity");
        if causal.len() == 2 {
            tests.push(Test::pass("TemporalBridge: causal root finds 2 memories"));
        } else {
            tests.push(Test::fail(
                "TemporalBridge: causal root",
                format!("got {}", causal.len()),
            ));
        }
    }

    // -- Memory record maps to cognitive content -------------------------
    {
        let record = MemoryRecord::new(
            MemoryLayer::Verbatim,
            serde_json::json!({"input": "hello"}),
            "session-001",
            "const-identity",
        );
        let content = record.to_cognitive_content();
        if content.contains(LAYER_VERBATIM) && content.contains("session-001") {
            tests.push(Test::pass(
                "MemoryRecord: cognitive content contains layer tag and session",
            ));
        } else {
            let preview_len = content.len().min(100);
            tests.push(Test::fail(
                "MemoryRecord: cognitive content",
                format!("got: {}", &content[..preview_len]),
            ));
        }
    }

    // -- Layer tag uniqueness --------------------------------------------
    {
        use std::collections::HashSet;
        let tags: HashSet<&str> = [
            LAYER_VERBATIM,
            LAYER_EPISODIC,
            LAYER_SEMANTIC,
            LAYER_RELATIONAL,
            LAYER_CAUSAL,
            LAYER_PROCEDURAL,
            LAYER_ANTICIPATORY,
            LAYER_IDENTITY,
        ]
        .iter()
        .copied()
        .collect();
        if tags.len() == 8 {
            tests.push(Test::pass("Constants: all 8 layer tags are unique"));
        } else {
            tests.push(Test::fail(
                "Constants: layer tag uniqueness",
                format!("only {} unique", tags.len()),
            ));
        }
    }

    // -- Print results ---------------------------------------------------
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
        println!("===================================================");
        std::process::exit(1);
    } else {
        println!("  Memory soul verified.");
        println!("  8 memory layers: OK");
        println!("  Verbatim write-ahead + SHA256: OK");
        println!("  Session tracking: OK");
        println!("  Identity accumulation: OK");
        println!("  Temporal bridge (B+ tree): OK");
        println!("  Phase 6 complete -- Hydra has a soul.");
        println!("===================================================");
    }
}
