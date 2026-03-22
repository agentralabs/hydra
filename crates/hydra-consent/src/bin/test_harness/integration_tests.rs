//! Integration tests: federation + consensus + consent chained.

use hydra_consensus::{ConsensusEngine, SharedBelief};
use hydra_consent::{ConsentEngine, ConsentScope};
use hydra_federation::{
    FederationEngine, FederationPeer, PeerAddress, PeerCapability, ScopeItem, test_fingerprint,
};

use super::Test;

pub fn run(tests: &mut Vec<Test>) {
    println!("\n── integration: federation + consensus + consent ────");

    let mut fed = FederationEngine::new("hydra-main");
    let mut consent = ConsentEngine::new();
    let mut consen = ConsensusEngine::new();

    // Register peer first so handshake can verify identity
    let peer = FederationPeer::new(
        "hydra-partner",
        "Partner Hydra",
        PeerAddress::new("hydra-partner.host:7474".to_string()),
        vec![PeerCapability::GenomeSharing {
            domains: vec!["engineering".into()],
        }],
        test_fingerprint("hydra-partner"),
    );
    fed.register_peer(peer).expect("should register peer");

    // Handshake with partner (creates session)
    let hs = fed
        .handshake(
            "hydra-partner",
            vec![ScopeItem::GenomeEntries {
                domain: "engineering".into(),
                max_count: 50,
            }],
            vec![ScopeItem::PatternDetection],
            0.78,
        )
        .expect("handshake should succeed");
    let session_id = &hs.session_id;

    // Grant consent for the session scope
    consent.grant(
        "hydra-partner",
        ConsentScope::GenomeSharing {
            domain: "engineering".into(),
            max_entries: 50,
        },
        None,
        30,
    );

    // Check consent before sharing
    let check = consent.check("hydra-partner", "genome:engineering");

    if check.is_ok() {
        let receipt = fed
            .record_sharing(
                session_id,
                "genome-share",
                "Sharing 8 engineering genome entries. Consent verified.",
            )
            .expect("should record sharing");

        consent
            .record_sharing(
                "hydra-partner",
                "genome:engineering",
                "8 genome entries shared under session",
                &receipt,
            )
            .expect("should record consent");

        tests.push(Test::pass(
            "Integration: consent verified -> share recorded in both logs",
        ));
    } else {
        tests.push(Test::fail("Integration: consent check", "denied"));
    }

    // Resolve a belief conflict via consensus
    let remote_belief = SharedBelief::new(
        "retry-strategy",
        "exponential backoff with jitter is optimal for engineering retries",
        0.85,
        40,
        vec![],
        "hydra-partner",
        0.0,
    );
    let r = consen
        .resolve(
            "retry-strategy",
            "exponential backoff with jitter is optimal for engineering retries",
            0.80,
            25,
            &remote_belief,
        )
        .expect("should resolve");

    if r.is_resolved() && r.provenance.len() == 2 {
        tests.push(Test::pass(
            "Integration: belief conflict resolved with provenance from both peers",
        ));
    } else {
        tests.push(Test::fail(
            "Integration: consensus",
            format!("resolved={} prov={}", r.is_resolved(), r.provenance.len()),
        ));
    }

    println!("  i  {}", fed.summary());
    println!("  i  {}", consent.summary());
    println!("  i  {}", consen.summary());
}
