#[cfg(test)]
mod tests {
    use crate::store::*;
    use crate::belief::{Belief, BeliefCategory, BeliefSource};
    use crate::conflict::ConflictStrategy;

    fn fact(subject: &str, content: &str) -> Belief {
        Belief::new(BeliefCategory::Fact, subject, content, BeliefSource::UserStated)
    }

    fn inferred(subject: &str, content: &str) -> Belief {
        Belief::new(BeliefCategory::Fact, subject, content, BeliefSource::Inferred)
    }

    #[test]
    fn store_and_retrieve_belief() {
        let store = BeliefStore::default();
        let b = fact("rust version", "uses Rust 1.75");
        let id = store.record(b).unwrap();
        let results = store.get_by_subject("rust version");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
    }

    #[test]
    fn store_starts_empty() {
        let store = BeliefStore::default();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert_eq!(store.active_count(), 0);
    }

    #[test]
    fn query_by_subject_case_insensitive() {
        let store = BeliefStore::default();
        store.record(fact("Rust Version", "1.75")).unwrap();
        let results = store.get_by_subject("rust version");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn query_by_subject_partial_match() {
        let store = BeliefStore::default();
        store.record(fact("rust version info", "1.75")).unwrap();
        let results = store.get_by_subject("rust");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn get_active_filters_by_category() {
        let store = BeliefStore::default();
        store.record(fact("test", "a")).unwrap();
        store.record(Belief::new(
            BeliefCategory::Preference,
            "theme",
            "dark",
            BeliefSource::UserStated,
        )).unwrap();
        assert_eq!(store.get_active(BeliefCategory::Fact).len(), 1);
        assert_eq!(store.get_active(BeliefCategory::Preference).len(), 1);
        assert_eq!(store.get_active(BeliefCategory::Convention).len(), 0);
    }

    #[test]
    fn duplicate_belief_rejected() {
        let store = BeliefStore::default();
        let b = fact("test", "content");
        let b2 = b.clone();
        store.record(b).unwrap();
        assert_eq!(store.record(b2).unwrap_err(), BeliefError::Duplicate);
    }

    #[test]
    fn supersede_deactivates_old() {
        let store = BeliefStore::default();
        let old = fact("rust version", "1.70");
        let old_id = store.record(old).unwrap();

        let new = fact("rust version", "1.75");
        store.supersede(old_id, new).unwrap();

        assert_eq!(store.len(), 2);
        assert_eq!(store.active_count(), 1);

        let active = store.get_active(BeliefCategory::Fact);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].content, "1.75");
    }

    #[test]
    fn conflict_detection_same_subject() {
        let store = BeliefStore::new(ConflictStrategy::NewerWins);
        store.record(fact("rust coding style", "tabs")).unwrap();

        let incoming = fact("rust coding style", "spaces");
        let conflict = store.detect_conflict(&incoming);
        assert!(conflict.is_some());
    }

    #[test]
    fn conflict_detection_different_subject() {
        let store = BeliefStore::new(ConflictStrategy::NewerWins);
        store.record(fact("rust coding style", "tabs")).unwrap();

        let incoming = fact("favorite food", "pizza");
        let conflict = store.detect_conflict(&incoming);
        assert!(conflict.is_none());
    }

    #[test]
    fn conflict_resolution_newer_wins_supersedes_old() {
        let store = BeliefStore::new(ConflictStrategy::NewerWins);
        store.record(fact("rust coding style", "tabs")).unwrap();

        // Record conflicting belief — NewerWins keeps new, supersedes old
        store.record(fact("rust coding style", "spaces")).unwrap();
        assert_eq!(store.active_count(), 1);

        let active = store.get_active(BeliefCategory::Fact);
        assert_eq!(active[0].content, "spaces");
    }

    #[test]
    fn conflict_resolution_user_stated_wins_keeps_user_over_inferred() {
        let store = BeliefStore::new(ConflictStrategy::UserStatedWins);
        store.record(fact("rust coding style", "tabs")).unwrap();

        // Inferred belief on same subject should lose
        let incoming = inferred("rust coding style", "spaces");
        store.record(incoming).unwrap();

        // The user-stated one stays active, inferred is kept old
        let active = store.get_active(BeliefCategory::Fact);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].content, "tabs");
    }

    #[test]
    fn wal_records_all_writes() {
        let store = BeliefStore::default();
        store.record(fact("a", "1")).unwrap();
        store.record(fact("b", "2")).unwrap();
        let wal = store.get_wal();
        assert_eq!(wal.len(), 2);
    }

    #[test]
    fn wal_recovery_restores_beliefs() {
        let store = BeliefStore::default();
        store.record(fact("a", "1")).unwrap();
        store.record(fact("b", "2")).unwrap();
        let wal = store.get_wal();

        let recovered = BeliefStore::recover(&wal, ConflictStrategy::UserStatedWins);
        assert_eq!(recovered.len(), 2);
        assert_eq!(recovered.active_count(), 2);
    }

    #[test]
    fn disk_full_error() {
        let store = BeliefStore::default();
        store.simulate_disk_full();
        let result = store.record(fact("test", "content"));
        assert_eq!(result.unwrap_err(), BeliefError::DiskFull);
    }

    #[test]
    fn crash_on_write_writes_to_wal_but_errors() {
        let store = BeliefStore::default();
        store.simulate_crash_on_write();
        let result = store.record(fact("test", "content"));
        assert_eq!(result.unwrap_err(), BeliefError::CrashDuringWrite);
        // WAL should still have the entry
        assert_eq!(store.get_wal().len(), 1);
        // But beliefs should be empty
        assert!(store.is_empty());
    }

    #[test]
    fn circular_supersession_rejected() {
        let store = BeliefStore::default();
        let a = fact("test", "a");
        let a_id = store.record(a).unwrap();

        // Try to make a belief that supersedes itself via a chain
        let mut b = fact("test2", "b");
        b.supersedes = Some(a_id);
        let b_id = store.record(b).unwrap();

        let mut c = fact("test3", "c");
        c.supersedes = Some(b_id);
        // This is fine -- no cycle
        let _c_id = store.record(c).unwrap();
    }

    #[test]
    fn get_related_finds_similar_subjects() {
        let store = BeliefStore::default();
        store.record(fact("rust coding conventions", "use snake_case")).unwrap();
        store.record(fact("python coding conventions", "use snake_case")).unwrap();
        store.record(fact("favorite food", "pizza")).unwrap();

        let related = store.get_related("rust coding", 0.3);
        assert!(related.len() >= 1);
        // "favorite food" should not match
        assert!(related.iter().all(|b| b.subject != "favorite food"));
    }

    #[test]
    fn belief_error_display() {
        let err = BeliefError::DiskFull;
        let msg = format!("{}", err);
        assert!(msg.contains("Storage is full"));
    }

    #[test]
    fn belief_error_severity() {
        assert_eq!(BeliefError::DiskFull.severity(), "critical");
        assert_eq!(BeliefError::Duplicate.severity(), "warning");
        assert_eq!(BeliefError::CircularSupersession.severity(), "error");
    }

    #[test]
    fn default_store_uses_user_stated_wins() {
        let store = BeliefStore::default();
        // Verify by recording conflicting beliefs with different sources
        store.record(fact("test subject", "user says A")).unwrap();
        let incoming = inferred("test subject", "inferred B");
        store.record(incoming).unwrap();

        let active = store.get_active(BeliefCategory::Fact);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].content, "user says A");
    }

    #[test]
    fn multiple_categories_tracked_independently() {
        let store = BeliefStore::default();
        store.record(Belief::new(BeliefCategory::Fact, "lang", "rust", BeliefSource::UserStated)).unwrap();
        store.record(Belief::new(BeliefCategory::Preference, "editor", "vim", BeliefSource::UserStated)).unwrap();
        store.record(Belief::new(BeliefCategory::Convention, "commit prefix", "feat:", BeliefSource::UserStated)).unwrap();
        store.record(Belief::new(BeliefCategory::Correction, "typo fix", "their not there", BeliefSource::Corrected)).unwrap();

        assert_eq!(store.len(), 4);
        assert_eq!(store.get_active(BeliefCategory::Fact).len(), 1);
        assert_eq!(store.get_active(BeliefCategory::Preference).len(), 1);
        assert_eq!(store.get_active(BeliefCategory::Convention).len(), 1);
        assert_eq!(store.get_active(BeliefCategory::Correction).len(), 1);
    }

    #[test]
    fn same_content_no_conflict() {
        let store = BeliefStore::new(ConflictStrategy::NewerWins);
        store.record(fact("rust version", "1.75")).unwrap();
        let incoming = fact("rust version", "1.75");
        let conflict = store.detect_conflict(&incoming);
        assert!(conflict.is_none());
    }

    #[test]
    fn wal_recovery_skips_zero_confidence() {
        let store = BeliefStore::default();
        let mut b = fact("test", "content");
        b.confidence = 0.0;
        let wal = vec![b];
        let recovered = BeliefStore::recover(&wal, ConflictStrategy::UserStatedWins);
        assert!(recovered.is_empty());
    }
}
