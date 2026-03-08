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
