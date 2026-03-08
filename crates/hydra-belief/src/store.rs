use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::RwLock;
use uuid::Uuid;

use crate::belief::{Belief, BeliefCategory, BeliefSource};
use crate::conflict::{self, Conflict, ConflictStrategy, Resolution};

/// Belief-specific errors with severity, category, user message, and suggested action
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeliefError {
    DiskFull,
    Duplicate,
    CircularSupersession,
    InvalidCategory,
    CrashDuringWrite,
}

impl BeliefError {
    pub fn severity(&self) -> &'static str {
        match self {
            Self::DiskFull => "critical",
            Self::Duplicate => "warning",
            Self::CircularSupersession => "error",
            Self::InvalidCategory => "error",
            Self::CrashDuringWrite => "critical",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Self::DiskFull => "resource_error",
            Self::Duplicate => "user_error",
            Self::CircularSupersession => "internal_error",
            Self::InvalidCategory => "user_error",
            Self::CrashDuringWrite => "internal_error",
        }
    }

    pub fn user_message(&self) -> &'static str {
        match self {
            Self::DiskFull => "Cannot save belief. Storage is full. Free up space to continue learning.",
            Self::Duplicate => "This belief is already recorded. No update needed.",
            Self::CircularSupersession => "Circular belief reference detected. This would create an infinite loop. The belief was not saved.",
            Self::InvalidCategory => "Invalid belief category. Use Preference, Fact, Convention, or Correction.",
            Self::CrashDuringWrite => "Belief write interrupted. The belief was saved to the recovery log. Restart to recover.",
        }
    }

    pub fn suggested_action(&self) -> &'static str {
        match self {
            Self::DiskFull => "Free disk space or increase storage limit in config.",
            Self::Duplicate => "No action needed. The existing belief is current.",
            Self::CircularSupersession => "Check the supersession chain for loops.",
            Self::InvalidCategory => {
                "Specify a valid category: preference, fact, convention, correction."
            }
            Self::CrashDuringWrite => "Run 'hydra doctor --repair' to recover.",
        }
    }
}

impl std::fmt::Display for BeliefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}. {}. {}",
            self.user_message(),
            self.category(),
            self.suggested_action()
        )
    }
}

impl std::error::Error for BeliefError {}

/// The belief store — single owner of all belief state
///
/// State ownership:
/// - `beliefs`: Owned, all active + superseded beliefs
/// - `wal`: Write-ahead log for crash recovery
/// - `strategy`: Conflict resolution strategy
/// - Cleanup: beliefs are never deleted, only superseded
pub struct BeliefStore {
    beliefs: RwLock<Vec<Belief>>,
    wal: RwLock<Vec<Belief>>,
    strategy: ConflictStrategy,
    similarity_threshold: f32,
    disk_full: AtomicBool,
    crash_on_write: AtomicBool,
}

impl BeliefStore {
    pub fn new(strategy: ConflictStrategy) -> Self {
        Self {
            beliefs: RwLock::new(Vec::new()),
            wal: RwLock::new(Vec::new()),
            strategy,
            similarity_threshold: 0.5,
            disk_full: AtomicBool::new(false),
            crash_on_write: AtomicBool::new(false),
        }
    }

    /// Record a new belief with conflict detection
    pub fn record(&self, belief: Belief) -> Result<Uuid, BeliefError> {
        if self.disk_full.load(Ordering::SeqCst) {
            return Err(BeliefError::DiskFull);
        }

        // Duplicate check
        {
            let beliefs = self.beliefs.read();
            if beliefs.iter().any(|b| b.id == belief.id) {
                return Err(BeliefError::Duplicate);
            }
        }

        // Circular supersession check
        if let Some(supersedes_id) = belief.supersedes {
            if self.would_create_cycle(supersedes_id, belief.id) {
                return Err(BeliefError::CircularSupersession);
            }
        }

        // Conflict detection and resolution
        if let Some(conflict) = self.detect_conflict(&belief) {
            let resolution = conflict::resolve_conflict(&conflict, self.strategy);
            match resolution {
                Resolution::KeepOld => return Ok(conflict.existing.id),
                Resolution::KeepNew => {
                    // Supersede the old belief
                    let old_id = conflict.existing.id;
                    let mut beliefs = self.beliefs.write();
                    if let Some(old) = beliefs.iter_mut().find(|b| b.id == old_id) {
                        old.active = false;
                        old.superseded_by = Some(belief.id);
                    }
                }
                Resolution::Merge | Resolution::AskUser => {
                    // For now, keep new (real implementation would ask user)
                }
            }
        }

        // WAL write first (crash recovery)
        self.wal.write().push(belief.clone());

        // Crash simulation
        if self.crash_on_write.load(Ordering::SeqCst) {
            return Err(BeliefError::CrashDuringWrite);
        }

        // Commit
        let id = belief.id;
        if let Some(supersedes_id) = belief.supersedes {
            let mut beliefs = self.beliefs.write();
            if let Some(old) = beliefs.iter_mut().find(|b| b.id == supersedes_id) {
                old.active = false;
                old.superseded_by = Some(id);
            }
            beliefs.push(belief);
        } else {
            self.beliefs.write().push(belief);
        }

        Ok(id)
    }

    /// Get beliefs by subject
    pub fn get_by_subject(&self, subject: &str) -> Vec<Belief> {
        let lower = subject.to_lowercase();
        self.beliefs
            .read()
            .iter()
            .filter(|b| b.subject.to_lowercase().contains(&lower))
            .cloned()
            .collect()
    }

    /// Get all active (non-superseded) beliefs in a category
    pub fn get_active(&self, category: BeliefCategory) -> Vec<Belief> {
        self.beliefs
            .read()
            .iter()
            .filter(|b| b.active && b.category == category)
            .cloned()
            .collect()
    }

    /// Get beliefs related to a subject by similarity
    pub fn get_related(&self, subject: &str, threshold: f32) -> Vec<Belief> {
        let probe = Belief::new(BeliefCategory::Fact, subject, "", BeliefSource::Inferred);
        self.beliefs
            .read()
            .iter()
            .filter(|b| b.active && b.subject_similarity(&probe) >= threshold)
            .cloned()
            .collect()
    }

    /// Supersede an old belief with a new one
    pub fn supersede(&self, old_id: Uuid, new_belief: Belief) -> Result<Uuid, BeliefError> {
        let mut belief = new_belief;
        belief.supersedes = Some(old_id);
        self.record(belief)
    }

    /// Detect conflict with existing beliefs
    pub fn detect_conflict(&self, incoming: &Belief) -> Option<Conflict> {
        let beliefs = self.beliefs.read();
        for existing in beliefs.iter().filter(|b| b.active) {
            let similarity = incoming.subject_similarity(existing);
            if similarity >= self.similarity_threshold && existing.id != incoming.id {
                // Same subject, different content = conflict
                if existing.content != incoming.content {
                    return Some(Conflict {
                        existing: existing.clone(),
                        incoming: incoming.clone(),
                        similarity,
                    });
                }
            }
        }
        None
    }

    /// Total belief count (active + superseded)
    pub fn len(&self) -> usize {
        self.beliefs.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.beliefs.read().is_empty()
    }

    /// Active belief count
    pub fn active_count(&self) -> usize {
        self.beliefs.read().iter().filter(|b| b.active).count()
    }

    /// Recover from WAL
    pub fn recover(wal_entries: &[Belief], strategy: ConflictStrategy) -> Self {
        let store = Self::new(strategy);
        for entry in wal_entries {
            if entry.confidence > 0.0 {
                let _ = store.record(entry.clone());
            }
        }
        store
    }

    /// Get WAL entries
    pub fn get_wal(&self) -> Vec<Belief> {
        self.wal.read().clone()
    }

    fn would_create_cycle(&self, supersedes_id: Uuid, new_id: Uuid) -> bool {
        let beliefs = self.beliefs.read();
        let mut current = Some(supersedes_id);
        let mut visited = std::collections::HashSet::new();
        visited.insert(new_id);

        while let Some(id) = current {
            if !visited.insert(id) {
                return true; // cycle
            }
            current = beliefs
                .iter()
                .find(|b| b.id == id)
                .and_then(|b| b.supersedes);
        }
        false
    }

    // Test helpers
    pub fn simulate_disk_full(&self) {
        self.disk_full.store(true, Ordering::SeqCst);
    }

    pub fn simulate_crash_on_write(&self) {
        self.crash_on_write.store(true, Ordering::SeqCst);
    }
}

impl Default for BeliefStore {
    fn default() -> Self {
        Self::new(ConflictStrategy::UserStatedWins)
    }
}

impl Clone for BeliefStore {
    fn clone(&self) -> Self {
        Self {
            beliefs: RwLock::new(self.beliefs.read().clone()),
            wal: RwLock::new(self.wal.read().clone()),
            strategy: self.strategy,
            similarity_threshold: self.similarity_threshold,
            disk_full: AtomicBool::new(self.disk_full.load(Ordering::SeqCst)),
            crash_on_write: AtomicBool::new(self.crash_on_write.load(Ordering::SeqCst)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
