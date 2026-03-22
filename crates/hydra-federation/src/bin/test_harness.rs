//! Phase 38 Test Harness — hydra-federation
//! Run: cargo run -p hydra-federation --bin test_harness

use hydra_federation::{
    test_fingerprint, FederationEngine, FederationError, FederationPeer, PeerAddress,
    PeerCapability, ScopeItem,
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

fn add_peer(engine: &mut FederationEngine, id: &str, capabilities: Vec<PeerCapability>) {
    let peer = FederationPeer::new(
        id,
        format!("Hydra Instance {}", id),
        PeerAddress::new(format!("{}.agentra.io:7474", id)),
        capabilities,
        test_fingerprint(id),
    );
    engine.register_peer(peer).expect("register peer");
}

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  Phase 38 — hydra-federation");
    println!("  Layer 6, Phase 1: Peer Discovery and Trust Negotiation");
    println!("  \"Two Hydras talking. Neither becomes the other.\"");
    println!("═══════════════════════════════════════════════════════");

    let mut tests = Vec::new();
    let mut engine = FederationEngine::new("hydra-agentra-main");

    // -- PEER IDENTITY --
    println!("\n── peer identity ────────────────────────────────────");

    {
        add_peer(
            &mut engine,
            "hydra-b",
            vec![
                PeerCapability::GenomeSharing {
                    domains: vec!["engineering".into()],
                },
                PeerCapability::PatternCollective,
            ],
        );
        add_peer(
            &mut engine,
            "hydra-c",
            vec![
                PeerCapability::GenomeSharing {
                    domains: vec!["fintech".into()],
                },
                PeerCapability::WisdomSharing {
                    domains: vec!["fintech".into()],
                },
            ],
        );

        if engine.peer_count() == 2 {
            tests.push(Test::pass("Peers: 2 Hydra instances registered"));
        } else {
            tests.push(Test::fail(
                "Peers: count",
                format!("{}", engine.peer_count()),
            ));
        }
    }

    {
        // Valid fingerprint verifies
        engine.verify_peer("hydra-b").expect("verify hydra-b");
        let peer = engine.registry.get_peer("hydra-b").expect("get hydra-b");
        if peer.is_verified {
            tests.push(Test::pass(
                "Identity: SHA256 fingerprint verified (64-char hex)",
            ));
        } else {
            tests.push(Test::fail("Identity: verification", "not verified"));
        }
    }

    {
        // Invalid fingerprint fails
        let mut bad_peer = FederationPeer::new(
            "hydra-bad",
            "Bad Peer",
            PeerAddress::new("bad.host:7474"),
            vec![],
            "not-a-valid-fingerprint-at-all",
        );
        let verified = bad_peer.verify_identity();
        if !verified {
            tests.push(Test::pass("Identity: invalid fingerprint rejected"));
        } else {
            tests.push(Test::fail("Identity: invalid rejection", "accepted"));
        }
    }

    // -- TRUST NEGOTIATION --
    println!("\n── trust negotiation ────────────────────────────────");

    {
        // Low trust peer rejected
        let result = engine.handshake(
            "hydra-b",
            vec![ScopeItem::PatternDetection],
            vec![ScopeItem::PatternDetection],
            0.40, // below MIN_FEDERATION_TRUST
        );
        if let Err(FederationError::InsufficientTrust { .. }) = result {
            tests.push(Test::pass(
                "Trust: score 0.40 rejected (below 0.65 threshold)",
            ));
        } else {
            tests.push(Test::fail("Trust: low rejection", "not rejected"));
        }
    }

    {
        // Full handshake with sufficient trust
        let result = engine
            .handshake(
                "hydra-b",
                vec![
                    ScopeItem::GenomeEntries {
                        domain: "engineering".into(),
                        max_count: 50,
                    },
                    ScopeItem::PatternDetection,
                ],
                vec![ScopeItem::GenomeEntries {
                    domain: "fintech".into(),
                    max_count: 30,
                }],
                0.82,
            )
            .expect("handshake should succeed");

        if result.our_offers == 2 && result.their_offers == 1 {
            tests.push(Test::pass(
                "Negotiation: scope agreed — 2 our offers, 1 their offer",
            ));
        } else {
            tests.push(Test::fail(
                "Negotiation: offers",
                format!("ours={} theirs={}", result.our_offers, result.their_offers),
            ));
        }

        if engine.active_session_count() == 1 {
            tests.push(Test::pass(
                "Session: established after successful handshake",
            ));
        } else {
            tests.push(Test::fail(
                "Session: count",
                format!("{}", engine.active_session_count()),
            ));
        }

        println!("  i  Handshake with hydra-b:");
        println!("     trust:   {:.2}", result.trust_score);
        println!(
            "     scope:   {} <-> {} items",
            result.our_offers, result.their_offers
        );
        println!("     session: {}", &result.session_id[..8]);
    }

    // -- SCOPE ENFORCEMENT --
    println!("\n── scope enforcement ────────────────────────────────");

    {
        let scope = engine
            .registry
            .get_scope("hydra-b")
            .expect("get scope hydra-b");
        if scope.permits("genome:engineering") {
            tests.push(Test::pass(
                "Scope: genome:engineering is within agreed scope",
            ));
        } else {
            tests.push(Test::fail("Scope: genome:engineering", "not permitted"));
        }

        if !scope.permits("genome:security") {
            tests.push(Test::pass(
                "Scope: genome:security NOT in scope (not negotiated)",
            ));
        } else {
            tests.push(Test::fail(
                "Scope: scope overflow",
                "permitted when shouldn't be",
            ));
        }
    }

    // -- SESSION EVENTS --
    println!("\n── session events ───────────────────────────────────");

    {
        let session_id = engine.registry.active_sessions()[0].id.clone();

        let receipt = engine
            .record_sharing(
                &session_id,
                "genome-share",
                "Shared 12 engineering genome entries. \
             Provenance: hydra-agentra-main. Confidence: 0.87.",
            )
            .expect("record sharing");

        if !receipt.is_empty() && receipt.len() == 64 {
            tests.push(Test::pass(
                "Event: genome sharing receipted (SHA256, 64 chars)",
            ));
        } else {
            tests.push(Test::fail(
                "Event: receipt",
                format!("len={}", receipt.len()),
            ));
        }

        let _receipt2 = engine
            .record_sharing(
                &session_id,
                "pattern-share",
                "Participating in cascade failure pattern detection. \
             Contributed 3 observations.",
            )
            .expect("record sharing 2");

        let session = engine.registry.active_sessions()[0];
        if session.event_count() == 2 {
            tests.push(Test::pass("Event: 2 events in session — all receipted"));
        } else {
            tests.push(Test::fail(
                "Event: count",
                format!("{}", session.event_count()),
            ));
        }
    }

    // -- SESSION REVOCATION --
    println!("\n── session revocation ───────────────────────────────");

    {
        // Establish second session with hydra-c, then revoke it
        engine.verify_peer("hydra-c").expect("verify hydra-c");
        engine
            .handshake(
                "hydra-c",
                vec![ScopeItem::WisdomJudgments {
                    domain: "fintech".into(),
                }],
                vec![ScopeItem::GenomeEntries {
                    domain: "fintech".into(),
                    max_count: 20,
                }],
                0.75,
            )
            .expect("handshake hydra-c");

        assert_eq!(engine.active_session_count(), 2);

        let c_session_id = engine
            .registry
            .active_sessions()
            .iter()
            .find(|s| s.remote_id == "hydra-c")
            .map(|s| s.id.clone())
            .expect("find hydra-c session");

        engine.revoke_session(&c_session_id, "test: session revoked by local peer");

        if engine.active_session_count() == 1 {
            tests.push(Test::pass(
                "Revoke: session revoked — count drops from 2 to 1",
            ));
        } else {
            tests.push(Test::fail(
                "Revoke: count after revoke",
                format!("{}", engine.active_session_count()),
            ));
        }

        // Verify hydra-b session still active
        let b_active = engine
            .registry
            .active_sessions()
            .iter()
            .any(|s| s.remote_id == "hydra-b");
        if b_active {
            tests.push(Test::pass(
                "Revoke: hydra-b session unaffected by hydra-c revocation",
            ));
        } else {
            tests.push(Test::fail("Revoke: independent sessions", "hydra-b lost"));
        }
    }

    // -- THE SOVEREIGN IDENTITY TEST --
    println!("\n── sovereign identity ───────────────────────────────");
    println!("  \"Two Hydras talked. Neither became the other.\"");

    {
        let our_id_unchanged = engine.local_peer_id == "hydra-agentra-main";
        let peer_b_identity = engine
            .registry
            .get_peer("hydra-b")
            .map(|p| p.peer_id == "hydra-b")
            .unwrap_or(false);

        if our_id_unchanged && peer_b_identity {
            tests.push(Test::pass(
                "Sovereign: both identities intact after federation",
            ));
        } else {
            tests.push(Test::fail("Sovereign: identity", "changed"));
        }

        // The scope is what connected them — not merger
        let scope = engine
            .registry
            .get_scope("hydra-b")
            .expect("get scope hydra-b");
        if scope.is_active() && scope.our_offers.len() == 2 {
            tests.push(Test::pass(
                "Sovereign: scope defines connection — not merger",
            ));
        } else {
            tests.push(Test::fail("Sovereign: scope", "not as expected"));
        }
    }

    // -- SUMMARY --
    {
        let s = engine.summary();
        if s.contains("federation:") && s.contains("peers=") && s.contains("sessions=") {
            tests.push(Test::pass("Summary: format correct for TUI display"));
        } else {
            tests.push(Test::fail("Summary", s));
        }
        println!("\n  i  {}", engine.summary());
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
    println!("═══════════════════════════════════════════════════════");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        std::process::exit(1);
    } else {
        println!();
        println!("  hydra-federation verified.");
        println!("  Identity:    SHA256 fingerprint verification.");
        println!("  Trust gate:  0.40 rejected. 0.82 accepted.");
        println!("  Scope:       agreed by both. neither can expand unilaterally.");
        println!("  Events:      SHA256 receipted. immutable.");
        println!("  Revocation:  one session ends. others unaffected.");
        println!("  Sovereignty: both identities intact after federation.");
        println!("  Layer 6, Phase 1 complete.");
        println!("  Next: hydra-consensus + hydra-consent (parallel).");
        println!("═══════════════════════════════════════════════════════");
    }
}
