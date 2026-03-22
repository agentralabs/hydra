//! WisdomMemory — historical judgments that compound into wisdom.
//! When a similar situation recurs: retrieve prior judgment and its outcome.

use crate::statement::WisdomStatement;
use serde::{Deserialize, Serialize};

/// The outcome of a past wisdom judgment — was it correct?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgmentOutcome {
    pub was_correct: bool,
    pub actual_outcome: String,
    pub confidence_gap: f64, // actual accuracy - predicted confidence
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

/// One entry in wisdom memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WisdomMemoryEntry {
    pub id: String,
    pub context_sig: String, // normalized context for matching
    pub domain: String,
    pub recommendation: String,
    pub confidence: f64,
    pub outcome: Option<JudgmentOutcome>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl WisdomMemoryEntry {
    pub fn from_statement(stmt: &WisdomStatement) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            context_sig: normalize_context(&stmt.context),
            domain: "general".to_string(),
            recommendation: stmt.recommendation.label().to_string(),
            confidence: stmt.confidence,
            outcome: None,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn record_outcome(&mut self, was_correct: bool, actual: impl Into<String>) {
        let gap = if was_correct {
            self.confidence - 1.0 // perfect outcome
        } else {
            self.confidence - 0.0 // wrong outcome
        };
        self.outcome = Some(JudgmentOutcome {
            was_correct,
            actual_outcome: actual.into(),
            confidence_gap: gap,
            recorded_at: chrono::Utc::now(),
        });
    }
}

/// The wisdom memory store.
#[derive(Default)]
pub struct WisdomMemory {
    pub(crate) entries: Vec<WisdomMemoryEntry>,
    db: Option<crate::persistence::WisdomDb>,
}

impl WisdomMemory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            db: None,
        }
    }

    /// Create a wisdom memory backed by SQLite persistence, loading existing entries.
    pub fn open() -> Self {
        match crate::persistence::WisdomDb::open() {
            Ok(db) => {
                let entries = db.load_all();
                eprintln!("hydra: wisdom loaded {} entries from db", entries.len());
                Self {
                    entries,
                    db: Some(db),
                }
            }
            Err(e) => {
                eprintln!("hydra: wisdom db open failed: {}, using in-memory", e);
                Self::new()
            }
        }
    }

    pub fn store(&mut self, stmt: &WisdomStatement) -> String {
        if self.entries.len() >= crate::constants::MAX_WISDOM_MEMORIES {
            self.entries.remove(0);
        }
        let entry = WisdomMemoryEntry::from_statement(stmt);
        let id = entry.id.clone();
        if let Some(ref db) = self.db {
            db.insert(&entry);
        }
        self.entries.push(entry);
        id
    }

    /// Find similar past judgments by context signature similarity.
    pub fn recall_similar(&self, context: &str) -> Vec<&WisdomMemoryEntry> {
        let sig = normalize_context(context);
        let threshold = crate::constants::MEMORY_RECALL_THRESHOLD;

        let mut matches: Vec<(&WisdomMemoryEntry, f64)> = self
            .entries
            .iter()
            .filter_map(|e| {
                let sim = context_similarity(&e.context_sig, &sig);
                if sim >= threshold {
                    Some((e, sim))
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        matches.into_iter().map(|(e, _)| e).collect()
    }

    pub fn record_outcome(&mut self, entry_id: &str, was_correct: bool, actual: impl Into<String>) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == entry_id) {
            entry.record_outcome(was_correct, actual);
        }
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn accuracy_for_domain(&self, domain: &str) -> Option<f64> {
        let resolved: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.domain == domain && e.outcome.is_some())
            .collect();
        if resolved.is_empty() {
            return None;
        }
        let correct = resolved
            .iter()
            .filter(|e| e.outcome.as_ref().map(|o| o.was_correct).unwrap_or(false))
            .count();
        Some(correct as f64 / resolved.len() as f64)
    }
}


fn normalize_context(ctx: &str) -> String {
    let mut words: Vec<&str> = ctx.split_whitespace().filter(|w| w.len() > 3).collect();
    words.sort();
    words.dedup();
    words.join(" ").to_lowercase()
}

fn context_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
    let inter = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();
    if union == 0 {
        return 0.0;
    }
    inter as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::WisdomInput;
    use crate::statement::WisdomStatement;

    fn make_stmt(context: &str) -> WisdomStatement {
        let input = WisdomInput::new(context, "engineering").with_base_confidence(0.80);
        WisdomStatement::synthesize(&input)
    }

    #[test]
    fn statement_stored_and_recalled() {
        let mut mem = WisdomMemory::new();
        let stmt = make_stmt("deploy auth service to production with rollback");
        mem.store(&stmt);
        assert_eq!(mem.count(), 1);
    }

    #[test]
    fn similar_context_recalled() {
        let mut mem = WisdomMemory::new();
        mem.store(&make_stmt("deploy auth service production rollback cert"));
        let similar = mem.recall_similar("deploy auth service production rollback cert rotation");
        assert!(!similar.is_empty());
    }

    #[test]
    fn outcome_recording() {
        let mut mem = WisdomMemory::new();
        let stmt = make_stmt("deploy service");
        let id = mem.store(&stmt);
        mem.record_outcome(&id, true, "deployment successful");
        let entry = mem
            .entries
            .iter()
            .find(|e| e.id == id)
            .expect("entry should exist");
        assert!(
            entry
                .outcome
                .as_ref()
                .expect("outcome should exist")
                .was_correct
        );
    }
}
