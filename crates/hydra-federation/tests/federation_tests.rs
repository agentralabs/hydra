use hydra_federation::protocol::HelloParams;
use hydra_federation::{
    ConflictStrategy, DelegatedTask, DiscoveryMethod, FederationMessage, FederationType,
    LoadBalanceStrategy, PeerCapabilities, PeerDiscovery, PeerInfo, PeerRegistry, ShareLevel,
    SharedSkill, SkillSharing, SyncEntry, SyncProtocol, TaskDelegation, TaskPriority, TrustLevel,
};

// === Full federation flow: discover → register → delegate → share → sync ===

#[test]
fn test_full_federation_flow() {
    // 1. Discover peers
    let discovery = PeerDiscovery::new(DiscoveryMethod::Manual(vec![
        "192.168.1.10:9000".into(),
        "192.168.1.20:9000".into(),
    ]));
    let discovered = discovery.discover();
    assert_eq!(discovered.len(), 2);

    // 2. Register peers
    let registry = PeerRegistry::new();
    for (i, d) in discovered.iter().enumerate() {
        registry.register(PeerInfo {
            id: format!("peer-{}", i),
            name: format!("hydra-{}", i),
            endpoint: d.endpoint.clone(),
            version: "0.1.0".into(),
            capabilities: PeerCapabilities {
                sisters: vec!["memory".into(), "codebase".into()],
                skills: vec!["file_read".into()],
                max_concurrent_tasks: 4,
                available_memory_mb: 1024,
                federation_types: vec![FederationType::Personal],
            },
            trust_level: TrustLevel::Trusted,
            federation_type: FederationType::Personal,
            last_seen: chrono::Utc::now().to_rfc3339(),
            active_tasks: 0,
        });
    }
    assert_eq!(registry.count(), 2);

    // 3. Delegate a task
    let delegation = TaskDelegation::new(LoadBalanceStrategy::LeastLoaded);
    let task = DelegatedTask {
        id: "t1".into(),
        description: "analyze code".into(),
        requirements: vec!["codebase".into()],
        priority: TaskPriority::Normal,
        max_duration_secs: 60,
    };
    let peer = delegation.find_peer(&task, &registry).unwrap();
    assert!(peer.has_capability("codebase"));

    // 4. Share skills
    let sharing = SkillSharing::new();
    sharing.offer(SharedSkill {
        id: "s1".into(),
        name: "git_flow".into(),
        version: "1.0.0".into(),
        signature: "git_add→git_commit→git_push".into(),
        owner_peer: "local".into(),
        share_level: ShareLevel::Full,
    });
    let skill = sharing.handle_request("s1", &peer).unwrap();
    assert_eq!(skill.name, "git_flow");

    // 5. Sync state
    let sync = SyncProtocol::default();
    sync.local_put("project", serde_json::json!("hydra"), "local");

    let remote_changes = vec![SyncEntry {
        key: "remote_data".into(),
        value: serde_json::json!("from_peer"),
        version: 5,
        timestamp: chrono::Utc::now().to_rfc3339(),
        origin_peer: peer.id.clone(),
    }];
    let report = sync.merge(remote_changes);
    assert_eq!(report.incoming_applied, 1);
}

#[test]
fn test_trust_enforcement() {
    let registry = PeerRegistry::new();

    // Unknown peer
    registry.register(PeerInfo {
        id: "stranger".into(),
        name: "unknown".into(),
        endpoint: "1.2.3.4:9000".into(),
        version: "0.1.0".into(),
        capabilities: PeerCapabilities {
            sisters: vec!["memory".into()],
            ..Default::default()
        },
        trust_level: TrustLevel::Unknown,
        federation_type: FederationType::Collective,
        last_seen: chrono::Utc::now().to_rfc3339(),
        active_tasks: 0,
    });

    // Can't delegate to unknown peer
    let delegation = TaskDelegation::default();
    let task = DelegatedTask {
        id: "t1".into(),
        description: "test".into(),
        requirements: vec!["memory".into()],
        priority: TaskPriority::Normal,
        max_duration_secs: 30,
    };
    assert!(delegation.find_peer(&task, &registry).is_err());

    // Can't share with unknown peer
    let sharing = SkillSharing::new();
    sharing.offer(SharedSkill {
        id: "s1".into(),
        name: "test".into(),
        version: "1.0.0".into(),
        signature: "test".into(),
        owner_peer: "local".into(),
        share_level: ShareLevel::Full,
    });
    let peer = registry.get("stranger").unwrap();
    assert!(sharing.handle_request("s1", &peer).is_err());
}

#[test]
fn test_sync_bidirectional() {
    let node_a = SyncProtocol::new(ConflictStrategy::LastWriteWins);
    let node_b = SyncProtocol::new(ConflictStrategy::LastWriteWins);

    // A writes
    node_a.local_put("shared_key", serde_json::json!("from_a"), "a");

    // B writes different key
    node_b.local_put("other_key", serde_json::json!("from_b"), "b");

    // Sync A→B
    let a_changes = node_a.changes_since(0);
    node_b.merge(a_changes);
    assert!(node_b.get("shared_key").is_some());

    // Sync B→A
    let b_changes = node_b.changes_since(0);
    node_a.merge(b_changes);
    assert!(node_a.get("other_key").is_some());

    // Both have both keys
    assert_eq!(node_a.entry_count(), 2);
    assert_eq!(node_b.entry_count(), 2);
}

#[test]
fn test_protocol_hello_roundtrip() {
    let msg = FederationMessage::hello(
        "peer-123",
        "my-hydra",
        "0.1.0",
        PeerCapabilities {
            sisters: vec!["memory".into(), "vision".into()],
            skills: vec!["analyze".into()],
            max_concurrent_tasks: 8,
            available_memory_mb: 2048,
            federation_types: vec![FederationType::Personal, FederationType::Team],
        },
    );

    let json = serde_json::to_string(&msg).unwrap();
    let restored: FederationMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.method, "federation.hello");

    let params: HelloParams = serde_json::from_value(restored.params).unwrap();
    assert_eq!(params.peer_id, "peer-123");
    assert_eq!(params.capabilities.sisters.len(), 2);
}
