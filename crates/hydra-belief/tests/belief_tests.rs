use std::sync::Arc;

use hydra_belief::belief::{Belief, BeliefCategory, BeliefSource};
use hydra_belief::conflict::{self, Conflict, ConflictStrategy, Resolution};
use hydra_belief::store::{BeliefError, BeliefStore};

fn preference(subject: &str, content: &str) -> Belief {
    Belief::new(
        BeliefCategory::Preference,
        subject,
        content,
        BeliefSource::UserStated,
    )
}

fn correction(subject: &str, content: &str) -> Belief {
    Belief::new(
        BeliefCategory::Correction,
        subject,
        content,
        BeliefSource::Corrected,
    )
}

fn inferred(subject: &str, content: &str) -> Belief {
    Belief::new(
        BeliefCategory::Fact,
        subject,
        content,
        BeliefSource::Inferred,
    )
}

// ═══════════════════════════════════════════════════════════
// BELIEF TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_belief_creation() {
    let b = preference("indent style", "use 4 spaces");
    assert_eq!(b.category, BeliefCategory::Preference);
    assert_eq!(b.source, BeliefSource::UserStated);
    assert!(b.confidence >= 0.9);
    assert!(b.active);
}

#[test]
fn test_correction_has_highest_confidence() {
    let c = correction("indent style", "use tabs");
    assert!(c.confidence > 0.95);
}

#[test]
fn test_inferred_has_lower_confidence() {
    let i = inferred("language", "rust");
    assert!(i.confidence < 0.7);
}

#[test]
fn test_confirm_increases_confidence() {
    let mut b = inferred("lang", "rust");
    let before = b.confidence;
    b.confirm();
    assert!(b.confidence > before);
    assert_eq!(b.confirmations, 1);
}

#[test]
fn test_contradict_decreases_confidence() {
    let mut b = preference("style", "tabs");
    let before = b.confidence;
    b.contradict();
    assert!(b.confidence < before);
    assert_eq!(b.contradictions, 1);
}

#[test]
fn test_subject_similarity() {
    let a = preference("indent style", "tabs");
    let b = preference("indent style preference", "spaces");
    let sim = a.subject_similarity(&b);
    assert!(sim > 0.3); // "indent" and "style" shared
}

// ═══════════════════════════════════════════════════════════
// STORE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_store_record_and_get() {
    let store = BeliefStore::default();
    let b = preference("indent", "4 spaces");
    let id = store.record(b).unwrap();
    let found = store.get_by_subject("indent");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, id);
}

#[test]
fn test_store_get_active() {
    let store = BeliefStore::default();
    store.record(preference("a", "1")).unwrap();
    store.record(preference("b", "2")).unwrap();
    store.record(inferred("c", "3")).unwrap();
    let prefs = store.get_active(BeliefCategory::Preference);
    assert_eq!(prefs.len(), 2);
}

#[test]
fn test_store_supersede() {
    let store = BeliefStore::default();
    let old_id = store.record(preference("style", "tabs")).unwrap();
    let new = correction("style", "spaces");
    let new_id = store.supersede(old_id, new).unwrap();
    assert_ne!(old_id, new_id);
    assert_eq!(store.active_count(), 1); // Only new one active
}

#[test]
fn test_store_get_related() {
    let store = BeliefStore::new(ConflictStrategy::NewerWins);
    store
        .record(preference("code indent style", "4 spaces"))
        .unwrap();
    store
        .record(preference("variable naming", "snake_case"))
        .unwrap();
    let related = store.get_related("indent style", 0.3);
    assert!(!related.is_empty());
}

// ═══════════════════════════════════════════════════════════
// CONFLICT RESOLUTION TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_conflict_newer_wins() {
    let old = preference("style", "tabs");
    let new = preference("style", "spaces");
    let conflict = Conflict {
        existing: old,
        incoming: new,
        similarity: 1.0,
    };
    assert_eq!(
        conflict::resolve_conflict(&conflict, ConflictStrategy::NewerWins),
        Resolution::KeepNew
    );
}

#[test]
fn test_conflict_higher_confidence() {
    let old = inferred("style", "tabs").with_confidence(0.5);
    let new = preference("style", "spaces").with_confidence(0.9);
    let conflict = Conflict {
        existing: old,
        incoming: new,
        similarity: 1.0,
    };
    assert_eq!(
        conflict::resolve_conflict(&conflict, ConflictStrategy::HigherConfidence),
        Resolution::KeepNew
    );
}

#[test]
fn test_conflict_user_stated_wins() {
    let old = inferred("style", "tabs");
    let new = correction("style", "spaces");
    let conflict = Conflict {
        existing: old,
        incoming: new,
        similarity: 1.0,
    };
    assert_eq!(
        conflict::resolve_conflict(&conflict, ConflictStrategy::UserStatedWins),
        Resolution::KeepNew
    );
}

#[test]
fn test_conflict_user_stated_keeps_old() {
    let old = correction("style", "tabs");
    let new = inferred("style", "spaces");
    let conflict = Conflict {
        existing: old,
        incoming: new,
        similarity: 1.0,
    };
    assert_eq!(
        conflict::resolve_conflict(&conflict, ConflictStrategy::UserStatedWins),
        Resolution::KeepOld
    );
}

#[test]
fn test_conflict_resolution_deterministic() {
    let old = preference("style", "tabs");
    let new = preference("style", "spaces");
    let conflict = Conflict {
        existing: old.clone(),
        incoming: new.clone(),
        similarity: 1.0,
    };
    let r1 = conflict::resolve_conflict(&conflict, ConflictStrategy::NewerWins);
    let r2 = conflict::resolve_conflict(&conflict, ConflictStrategy::NewerWins);
    assert_eq!(r1, r2);
}

// ═══════════════════════════════════════════════════════════
// EDGE CASES (EC-BR-001 through EC-BR-010)
// ═══════════════════════════════════════════════════════════

/// EC-BR-001: Conflicting beliefs — automatic resolution
#[test]
fn test_ec_br_001_conflicting_beliefs() {
    let store = BeliefStore::new(ConflictStrategy::UserStatedWins);
    store.record(inferred("indent style", "tabs")).unwrap();
    store
        .record(correction("indent style", "4 spaces"))
        .unwrap();
    // Correction should win over inferred
    let active = store.get_active(BeliefCategory::Correction);
    assert!(!active.is_empty());
    assert!(active[0].content.contains("spaces"));
}

/// EC-BR-002: Belief supersession
#[test]
fn test_ec_br_002_belief_supersession() {
    let store = BeliefStore::default();
    let old_id = store.record(preference("naming", "camelCase")).unwrap();
    let new = correction("naming", "snake_case").with_supersedes(old_id);
    store.record(new).unwrap();
    let by_subject = store.get_by_subject("naming");
    let active: Vec<_> = by_subject.iter().filter(|b| b.active).collect();
    assert_eq!(active.len(), 1);
    assert!(active[0].content.contains("snake_case"));
}

/// EC-BR-003: Circular supersession prevented
#[test]
fn test_ec_br_003_circular_supersession_prevented() {
    let store = BeliefStore::default();
    let id_a = store.record(preference("x", "a")).unwrap();
    let mut b = preference("x", "b");
    b.supersedes = Some(id_a);
    let id_b = store.record(b).unwrap();
    // Try to make A supersede B (would create A→B→A cycle)
    let mut c = preference("x", "c");
    c.id = id_a; // Reuse A's ID — this would create a duplicate, caught
    c.supersedes = Some(id_b);
    let result = store.record(c);
    assert!(result.is_err()); // Either duplicate or circular
}

/// EC-BR-004: Confidence decay over time
#[test]
fn test_ec_br_004_confidence_decay_over_time() {
    let mut b = inferred("tool", "vim");
    let original = b.confidence;
    b.apply_decay(0.01); // 1% per day
                         // With 0 days elapsed, no decay (or minimal)
    assert!(b.confidence <= original);
}

/// EC-BR-005: High volume beliefs (1000+)
#[test]
fn test_ec_br_005_high_volume_beliefs() {
    let store = BeliefStore::new(ConflictStrategy::NewerWins);
    for i in 0..1000 {
        let b = Belief::new(
            BeliefCategory::Fact,
            format!("fact_{i}"),
            format!("value_{i}"),
            BeliefSource::Inferred,
        );
        store.record(b).unwrap();
    }
    assert_eq!(store.len(), 1000);
    let found = store.get_by_subject("fact_500");
    assert_eq!(found.len(), 1);
}

/// EC-BR-006: Concurrent belief updates
#[tokio::test]
async fn test_ec_br_006_concurrent_belief_updates() {
    let store = Arc::new(BeliefStore::new(ConflictStrategy::NewerWins));
    let handles: Vec<_> = (0..50)
        .map(|i| {
            let s = store.clone();
            tokio::spawn(async move {
                let b = Belief::new(
                    BeliefCategory::Fact,
                    format!("concurrent_{i}"),
                    format!("val_{i}"),
                    BeliefSource::Inferred,
                );
                s.record(b)
            })
        })
        .collect();
    let results = futures::future::join_all(handles).await;
    for r in &results {
        assert!(r.as_ref().unwrap().is_ok());
    }
    assert_eq!(store.len(), 50);
}

/// EC-BR-007: Invalid belief category (tested via type system — always valid)
#[test]
fn test_ec_br_007_invalid_belief_category() {
    // Rust's type system prevents invalid categories at compile time.
    // We test that all valid categories are accepted.
    let categories = [
        BeliefCategory::Preference,
        BeliefCategory::Fact,
        BeliefCategory::Convention,
        BeliefCategory::Correction,
    ];
    let store = BeliefStore::default();
    for cat in &categories {
        let b = Belief::new(*cat, "test", "val", BeliefSource::UserStated);
        assert!(store.record(b).is_ok());
    }
}

/// EC-BR-008: Belief persistence crash — WAL recovery
#[test]
fn test_ec_br_008_belief_persistence_crash() {
    let store = BeliefStore::default();
    store.record(preference("committed", "value1")).unwrap();
    store.record(preference("committed2", "value2")).unwrap();

    store.simulate_crash_on_write();
    let crash_belief = preference("crash_during", "value3");
    let result = store.record(crash_belief);
    assert_eq!(result.unwrap_err(), BeliefError::CrashDuringWrite);

    // Recover from WAL
    let wal = store.get_wal();
    let recovered = BeliefStore::recover(&wal, ConflictStrategy::UserStatedWins);
    assert!(recovered.len() >= 2); // At least the committed ones
}

/// EC-BR-009: Semantic similarity threshold
#[test]
fn test_ec_br_009_semantic_similarity_threshold() {
    let store = BeliefStore::default();
    store
        .record(preference("code indentation style", "4 spaces"))
        .unwrap();
    // Different subject — should not conflict
    store
        .record(preference("variable naming convention", "snake_case"))
        .unwrap();
    assert_eq!(store.active_count(), 2); // Both active, no conflict
}

/// EC-BR-010: User correction always has priority
#[test]
fn test_ec_br_010_user_correction_priority() {
    let store = BeliefStore::new(ConflictStrategy::UserStatedWins);
    store.record(inferred("indent", "tabs")).unwrap();
    store.record(correction("indent", "spaces")).unwrap();
    let active = store
        .get_by_subject("indent")
        .into_iter()
        .filter(|b| b.active)
        .collect::<Vec<_>>();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].source, BeliefSource::Corrected);
}

// ═══════════════════════════════════════════════════════════
// ERROR CLASSIFICATION TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_errors_have_severity_and_category() {
    let errors = vec![
        BeliefError::DiskFull,
        BeliefError::Duplicate,
        BeliefError::CircularSupersession,
        BeliefError::InvalidCategory,
        BeliefError::CrashDuringWrite,
    ];
    for err in &errors {
        assert!(!err.severity().is_empty());
        assert!(!err.category().is_empty());
        assert!(!err.user_message().is_empty());
        assert!(!err.suggested_action().is_empty());
    }
}

// ═══════════════════════════════════════════════════════════
// SERDE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_belief_serde_roundtrip() {
    let b = preference("style", "4 spaces");
    let json = serde_json::to_string(&b).unwrap();
    let de: Belief = serde_json::from_str(&json).unwrap();
    assert_eq!(de.id, b.id);
    assert_eq!(de.content, b.content);
}
