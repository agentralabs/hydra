//! Category 5: Stress — concurrency tests.

use std::sync::Arc;
use std::thread;

#[test]
fn test_concurrent_checkpoint_saves() {
    use hydra_inventions::resurrection::store::CheckpointStore;
    use hydra_inventions::resurrection::checkpoint::Checkpoint;

    let store = Arc::new(CheckpointStore::new());
    let handles: Vec<_> = (0..50).map(|i| {
        let store = store.clone();
        thread::spawn(move || {
            let cp = Checkpoint::create(
                &format!("cp_{}", i),
                serde_json::json!({"index": i}),
            );
            store.save(cp);
        })
    }).collect();

    for h in handles { h.join().unwrap(); }
    assert_eq!(store.count(), 50);
}

#[test]
fn test_concurrent_event_bus() {
    use hydra_runtime::*;
    let bus = Arc::new(EventBus::new(1000));

    let handles: Vec<_> = (0..20).map(|_| {
        let bus = bus.clone();
        thread::spawn(move || {
            for _ in 0..50 {
                bus.publish(SseEvent::heartbeat());
            }
        })
    }).collect();

    for h in handles { h.join().unwrap(); }
    assert!(bus.total_published() >= 1000);
}

#[test]
fn test_concurrent_registry_access() {
    use hydra_model::registry::ModelRegistry;
    use hydra_model::profile::*;

    let registry = Arc::new(ModelRegistry::new());

    let handles: Vec<_> = (0..20).map(|i| {
        let registry = registry.clone();
        thread::spawn(move || {
            for _ in 0..10 {
                let _ = registry.list_available();
                let _ = registry.count();
                if i % 2 == 0 {
                    let _ = registry.get("claude-opus");
                }
            }
        })
    }).collect();

    for h in handles { h.join().unwrap(); }
}

#[test]
fn test_concurrent_pattern_tracking() {
    use hydra_inventions::mutation::tracker::PatternTracker;

    let tracker = Arc::new(PatternTracker::new());
    tracker.register("pattern1", vec!["a".into(), "b".into()]);

    let handles: Vec<_> = (0..30).map(|i| {
        let tracker = tracker.clone();
        thread::spawn(move || {
            for _ in 0..20 {
                tracker.record("pattern1", i % 3 != 0, 100);
            }
        })
    }).collect();

    for h in handles { h.join().unwrap(); }
    let pat = tracker.get("pattern1").unwrap();
    assert_eq!(pat.total_executions, 600);
}

#[test]
fn test_parallel_kill_switch() {
    use hydra_runtime::KillSwitch;

    let ks = Arc::new(KillSwitch::new());

    // Multiple threads checking kill switch state
    let handles: Vec<_> = (0..20).map(|_| {
        let ks = ks.clone();
        thread::spawn(move || {
            for _ in 0..100 {
                let _ = ks.is_active();
                let _ = ks.is_frozen();
                let _ = ks.should_block();
            }
        })
    }).collect();

    for h in handles { h.join().unwrap(); }
}
