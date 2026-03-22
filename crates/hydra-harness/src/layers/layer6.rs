//! Tests all Layer 6 capabilities.

use crate::TestResult;
use std::time::Instant;

pub fn run() -> Vec<TestResult> {
    let mut results = Vec::new();

    results.extend(test_federation());
    results.extend(test_consensus());
    results.extend(test_consent());
    results.extend(test_collective());
    results.extend(test_diplomat());

    let passed = results.iter().filter(|r| r.passed).count();
    println!("  Layer 6: {}/{} passed", passed, results.len());
    results
}

fn test_federation() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_federation::{
            FederationEngine, FederationPeer, PeerAddress, PeerCapability,
            ScopeItem, test_fingerprint,
        };
        let mut engine = FederationEngine::new("hydra-harness-local");
        let peer = FederationPeer::new(
            "hydra-harness-remote", "Test Peer",
            PeerAddress::new("test.host:7474"),
            vec![PeerCapability::PatternCollective],
            test_fingerprint("hydra-harness-remote"),
        );
        engine.register_peer(peer).expect("register must succeed");
        let result = engine.handshake(
            "hydra-harness-remote",
            vec![ScopeItem::PatternDetection],
            vec![ScopeItem::PatternDetection],
            0.80,
        ).expect("handshake must succeed");
        assert!(!result.session_id.is_empty());
        assert!(engine.active_session_count() == 1);
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-federation: peer handshake");
            results.push(TestResult::pass("hydra-federation", "peer_handshake",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-federation: {}", err);
            results.push(TestResult::fail("hydra-federation", "peer_handshake",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_consensus() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_consensus::{ConsensusEngine, SharedBelief};
        let mut engine = ConsensusEngine::new();
        let remote = SharedBelief::new(
            "circuit-breaker-pattern",
            "circuit breakers prevent cascade failures",
            0.85, 30, vec![], "peer-b", 0.0,
        );
        let resolution = engine.resolve(
            "circuit-breaker-pattern",
            "circuit breakers prevent cascade failures",
            0.80, 20,
            &remote,
        ).expect("resolve must succeed");
        assert!(resolution.is_resolved(), "Agreeing beliefs must resolve");
        assert_eq!(resolution.provenance.len(), 2,
            "Must have 2-source provenance");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-consensus: belief resolution");
            results.push(TestResult::pass("hydra-consensus", "belief_resolution",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-consensus: {}", err);
            results.push(TestResult::fail("hydra-consensus", "belief_resolution",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_consent() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_consent::{ConsentEngine, ConsentScope, ConsentError};
        let mut engine = ConsentEngine::new();
        engine.grant("peer-b", ConsentScope::PatternParticipation, None, 30);
        // Check: within scope
        assert!(engine.check("peer-b", "pattern").is_ok());
        // Check: no consent -> hard stop
        let r = engine.check("unknown-peer", "pattern");
        assert!(matches!(r, Err(ConsentError::NoConsent { .. })),
            "No consent must be a hard stop");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-consent: consent gate");
            results.push(TestResult::pass("hydra-consent", "consent_gate",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-consent: {}", err);
            results.push(TestResult::fail("hydra-consent", "consent_gate",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_collective() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_collective::{CollectiveEngine, PatternObservation};
        let mut engine = CollectiveEngine::new();
        for (peer, trust, count) in [
            ("p1", 0.85, 5usize), ("p2", 0.78, 8), ("p3", 0.90, 6)
        ] {
            engine.contribute(PatternObservation::new(
                "cascade-failure", peer, trust, 0.85, count,
                "cascade observed", "engineering",
            )).expect("contribute must succeed");
        }
        let insight = engine.produce_insight(
            "cascade-failure",
            "Cascade detected",
            "Install circuit breakers",
        ).expect("produce_insight must succeed");
        assert!(insight.peer_count == 3);
        assert!(insight.aggregated_confidence >= 0.65);
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-collective: trust-weighted insight");
            results.push(TestResult::pass("hydra-collective",
                "trust_weighted_insight", start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-collective: {}", err);
            results.push(TestResult::fail("hydra-collective",
                "trust_weighted_insight", &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_diplomat() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_diplomat::{DiplomatEngine, Stance};
        let mut engine = DiplomatEngine::new();
        let sid = engine.open_session("harness-coordination");
        for (peer, pos) in &[
            ("hydra-a", "use circuit breakers at service boundaries"),
            ("hydra-b", "use circuit breakers at service boundaries"),
        ] {
            engine.submit_stance(&sid, Stance::new(
                *peer, "harness-coordination", *pos, 0.85, vec![], vec![],
            )).expect("submit_stance must succeed");
        }
        let rec = engine.synthesize(&sid).expect("synthesize must succeed");
        assert!(rec.is_consensus(), "Agreeing stances must produce consensus");
        assert!(rec.minority_positions.is_empty());
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-diplomat: multi-peer coordination");
            results.push(TestResult::pass("hydra-diplomat",
                "multi_peer_coordination", start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-diplomat: {}", err);
            results.push(TestResult::fail("hydra-diplomat",
                "multi_peer_coordination", &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}
