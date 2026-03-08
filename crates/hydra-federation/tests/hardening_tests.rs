//! Category 1: Unit Gap Fill — hydra-federation edge cases.

use hydra_federation::*;

// === Peer disconnect mid-operation ===

#[test]
fn test_peer_registry_remove_nonexistent() {
    let registry = PeerRegistry::new();
    assert!(!registry.remove("nonexistent"));
}

#[test]
fn test_peer_registry_set_trust_not_found() {
    let registry = PeerRegistry::new();
    assert!(registry
        .set_trust("nonexistent", TrustLevel::Trusted)
        .is_err());
}

// === Sync conflict all strategies ===

#[test]
fn test_sync_last_write_wins() {
    let sync = SyncProtocol::new(ConflictStrategy::LastWriteWins);
    sync.local_put("key1", serde_json::json!("local_value"), "local");

    let remote = vec![SyncEntry {
        key: "key1".into(),
        value: serde_json::json!("remote_value"),
        version: 2,
        origin_peer: "remote".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }];
    let report = sync.merge(remote);
    assert!(report.conflicts_resolved > 0);
}

#[test]
fn test_sync_keep_local() {
    let sync = SyncProtocol::new(ConflictStrategy::KeepLocal);
    sync.local_put("key1", serde_json::json!("local"), "local");

    let remote = vec![SyncEntry {
        key: "key1".into(),
        value: serde_json::json!("remote"),
        version: 1,
        origin_peer: "remote".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }];
    let _report = sync.merge(remote);
    let entry = sync.get("key1").unwrap();
    // Local should be kept
    assert_eq!(entry.origin_peer, "local");
}

#[test]
fn test_sync_higher_version() {
    let sync = SyncProtocol::new(ConflictStrategy::HigherVersion);
    sync.local_put("key1", serde_json::json!("v1"), "local");
    let remote = vec![SyncEntry {
        key: "key1".into(),
        value: serde_json::json!("v2"),
        version: 999,
        origin_peer: "remote".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }];
    let report = sync.merge(remote);
    let _ = report;
}

// === Trust downgrade ===

#[test]
fn test_trust_level_ordering() {
    assert!(TrustLevel::Owner > TrustLevel::Trusted);
    assert!(TrustLevel::Trusted > TrustLevel::Known);
    assert!(TrustLevel::Known > TrustLevel::Unknown);
}

#[test]
fn test_trust_downgrade() {
    let registry = PeerRegistry::new();
    let peer = PeerInfo {
        id: "p1".into(),
        name: "peer1".into(),
        endpoint: "localhost:8080".into(),
        version: "0.1.0".into(),
        trust_level: TrustLevel::Trusted,
        federation_type: FederationType::Personal,
        capabilities: PeerCapabilities {
            sisters: vec![],
            skills: vec![],
            max_concurrent_tasks: 5,
            available_memory_mb: 512,
            federation_types: vec![FederationType::Personal],
        },
        active_tasks: 0,
        last_seen: chrono::Utc::now().to_rfc3339(),
    };
    registry.register(peer);
    assert!(registry.set_trust("p1", TrustLevel::Unknown).is_ok());
    let p = registry.get("p1").unwrap();
    assert_eq!(p.trust_level, TrustLevel::Unknown);
}

// === Delegation ===

#[test]
fn test_delegation_no_peers() {
    let delegation = TaskDelegation::new(LoadBalanceStrategy::LeastLoaded);
    let registry = PeerRegistry::new();
    let task = DelegatedTask {
        id: "t1".into(),
        description: "test".into(),
        requirements: vec!["code".into()],
        priority: TaskPriority::Normal,
        max_duration_secs: 60,
    };
    assert!(delegation.find_peer(&task, &registry).is_err());
}

// === Skill sharing ===

#[test]
fn test_sharing_insufficient_trust() {
    let sharing = SkillSharing::new();
    let skill = SharedSkill {
        id: "s1".into(),
        name: "test_skill".into(),
        version: "1.0".into(),
        signature: "test".into(),
        owner_peer: "owner".into(),
        share_level: ShareLevel::Full,
    };
    sharing.offer(skill);

    // Set policy with default Private level
    sharing.set_policy(SharingPolicy {
        default_level: ShareLevel::Private,
        skill_overrides: std::collections::HashMap::new(),
        peer_overrides: std::collections::HashMap::new(),
    });

    let untrusted_peer = PeerInfo {
        id: "p1".into(),
        name: "untrusted".into(),
        endpoint: "localhost:9090".into(),
        version: "0.1.0".into(),
        trust_level: TrustLevel::Unknown,
        federation_type: FederationType::Collective,
        capabilities: PeerCapabilities::default(),
        active_tasks: 0,
        last_seen: chrono::Utc::now().to_rfc3339(),
    };
    let result = sharing.handle_request("s1", &untrusted_peer);
    assert!(result.is_err());
}

// === Discovery ===

#[test]
fn test_discovery_manual_add() {
    let discovery = PeerDiscovery::new(DiscoveryMethod::Manual(vec![]));
    discovery.add_manual("localhost:8080");
    assert_eq!(discovery.count(), 1);
    let peers = discovery.cached();
    assert_eq!(peers.len(), 1);
}

// === Federation message ===

#[test]
fn test_federation_message_hello() {
    let msg = FederationMessage::hello(
        "peer1",
        "Test Peer",
        "1.0.0",
        PeerCapabilities {
            sisters: vec![],
            skills: vec!["code".into()],
            max_concurrent_tasks: 5,
            available_memory_mb: 512,
            federation_types: vec![FederationType::Personal],
        },
    );
    assert_eq!(msg.method, "federation.hello");
}

#[test]
fn test_federation_response_success_error() {
    let success = FederationResponse::success("1", serde_json::json!({"ok": true}));
    assert!(success.error.is_none());

    let error = FederationResponse::error("1", "something failed");
    assert!(error.error.is_some());
}
