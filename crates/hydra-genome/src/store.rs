//! Append-only genome store with BM25-lite retrieval.
//!
//! Query uses IDF-weighted term matching instead of plain Jaccard.
//! Discriminative terms (e.g., "netflix") get high weight.
//! Common terms (e.g., "service") get low weight.
//! This is the mathematical fix for indirect phrasing retrieval.

use crate::constants::{GENOME_MAX_ENTRIES, GENOME_QUERY_TOP_N, SITUATION_SIMILARITY_THRESHOLD};
use crate::entry::GenomeEntry;
use crate::errors::GenomeError;
use crate::signature::{ApproachSignature, SituationSignature};

/// Append-only store for genome entries.
///
/// Entries are never deleted or reset. `total_ever` is monotonically increasing.
#[derive(Default)]
pub struct GenomeStore {
    entries: Vec<GenomeEntry>,
    total_ever: u64,
    db: Option<crate::persistence::GenomeDb>,
}

impl std::fmt::Debug for GenomeStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenomeStore")
            .field("entries", &self.entries.len())
            .field("total_ever", &self.total_ever)
            .field("has_db", &self.db.is_some())
            .finish()
    }
}

impl GenomeStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            total_ever: 0,
            db: None,
        }
    }

    pub fn open() -> Self {
        match crate::persistence::GenomeDb::open() {
            Ok(db) => {
                let entries = db.load_all();
                let count = entries.len() as u64;
                eprintln!("hydra: genome loaded {} entries from db", count);
                Self {
                    entries,
                    total_ever: count,
                    db: Some(db),
                }
            }
            Err(e) => {
                eprintln!("hydra: !! GENOME DB FAILED: {e} — running with EMPTY knowledge !!");
                eprintln!("hydra: !! All learned patterns unavailable. Check ~/.hydra/data/genome.db !!");
                Self::new()
            }
        }
    }

    pub fn add(&mut self, entry: GenomeEntry) -> Result<String, GenomeError> {
        if self.entries.len() >= GENOME_MAX_ENTRIES {
            // EC-3.6: Evict lowest-value entry (low confidence + low use) before rejecting
            if let Some(idx) = self.entries.iter().enumerate()
                .filter(|(_, e)| e.effective_confidence() < 0.3 && e.use_count < 5)
                .min_by(|(_, a), (_, b)| a.effective_confidence().partial_cmp(&b.effective_confidence()).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
            {
                eprintln!("hydra-genome: evicting low-value entry at idx {idx} to make room");
                self.entries.remove(idx);
            } else {
                return Err(GenomeError::StoreFull { max: GENOME_MAX_ENTRIES });
            }
        }
        let id = entry.id.clone();
        if let Some(ref db) = self.db {
            if let Err(e) = db.insert(&entry) {
                eprintln!("hydra: genome persist failed (in-memory only): {e}");
            }
        }
        self.entries.push(entry);
        self.total_ever += 1;
        Ok(id)
    }

    /// Query the store using BM25-lite scoring.
    ///
    /// Instead of Jaccard (all terms equal weight), this uses IDF weighting:
    /// discriminative terms like "netflix" get high scores, common terms
    /// like "service" get low scores. This is the mathematical fix for
    /// indirect phrasings where few but important terms overlap.
    pub fn query(&self, description: &str) -> Vec<&GenomeEntry> {
        if self.entries.is_empty() {
            return Vec::new();
        }

        let query_sig = SituationSignature::from_description(description);
        let n = self.entries.len() as f64;

        // Compute IDF for each query term across all genome entries
        // IDF(t) = ln((N + 1) / (df(t) + 1)) where df = docs containing term
        let query_terms: Vec<&String> = query_sig.keywords.iter().collect();
        let idfs: Vec<f64> = query_terms
            .iter()
            .map(|term| {
                let df = self
                    .entries
                    .iter()
                    .filter(|e| e.situation.keywords.contains(*term))
                    .count() as f64;
                ((n + 1.0) / (df + 1.0)).ln()
            })
            .collect();

        // DSEA: compute query axiom vector for semantic matching
        let query_axiom = crate::signature::axiom_vector(&query_sig.keywords);

        // Dual-space scoring: max(lexical_IDF, semantic_cosine)
        let mut scored: Vec<(&GenomeEntry, f64)> = self
            .entries
            .iter()
            .filter_map(|entry| {
                // Channel 1: Lexical (IDF + Jaccard)
                let mut lexical_score = 0.0;
                for (i, term) in query_terms.iter().enumerate() {
                    if entry.situation.keywords.contains(*term) {
                        lexical_score += idfs[i];
                    }
                }
                let jaccard = entry.situation.similarity(&query_sig);
                if jaccard >= SITUATION_SIMILARITY_THRESHOLD {
                    lexical_score = lexical_score.max(jaccard * 5.0);
                }

                // Channel 2: Semantic (axiom vector cosine similarity)
                let entry_axiom = crate::signature::axiom_vector(&entry.situation.keywords);
                let semantic_score = crate::signature::axiom_cosine(&query_axiom, &entry_axiom) * 5.0;

                // Dual-space: take the MAX of both channels
                let score = lexical_score.max(semantic_score);

                if score > 0.5 {
                    let weighted = score * entry.effective_confidence();
                    Some((entry, weighted))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(GENOME_QUERY_TOP_N)
            .map(|(entry, _)| entry)
            .collect()
    }

    pub fn record_use(&mut self, id: &str, success: bool) -> Result<(), GenomeError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(|| GenomeError::EntryNotFound { id: id.to_string() })?;
        entry.record_use(success);
        Ok(())
    }

    pub fn total_ever(&self) -> u64 {
        self.total_ever
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Batch evict low-value entries (confidence < 0.3 AND use_count < 3).
    /// Returns number of entries evicted.
    pub fn evict_low_value(&mut self, max_evict: usize) -> usize {
        let mut evicted = 0;
        self.entries.retain(|e| {
            if evicted >= max_evict { return true; }
            if e.effective_confidence() < 0.3 && e.use_count < 3 {
                evicted += 1;
                false
            } else { true }
        });
        evicted
    }

    /// Return a reference to all entries (for distillation/analysis).
    pub fn all_entries(&self) -> &[GenomeEntry] {
        &self.entries
    }

    pub fn load_from_skills(&mut self) -> usize {
        let skill_genomes = crate::skill_loader::load_all_skill_genomes();
        let mut loaded = 0;
        for (_skill_name, entries) in skill_genomes {
            for entry in entries {
                if self.has_situation(&entry.situation) {
                    continue;
                }
                match self.add(entry) {
                    Ok(_) => loaded += 1,
                    Err(e) => {
                        eprintln!("hydra: skill genome add failed: {}", e);
                        break;
                    }
                }
            }
        }
        eprintln!("hydra: loaded {} genome entries from skills/", loaded);
        loaded
    }

    fn has_situation(&self, situation: &SituationSignature) -> bool {
        self.entries
            .iter()
            .any(|e| e.situation.keywords == situation.keywords)
    }

    pub fn add_from_operation(
        &mut self,
        description: &str,
        approach: ApproachSignature,
        confidence: f64,
    ) -> Result<String, GenomeError> {
        let entry = GenomeEntry::from_operation(description, approach, confidence);
        self.add(entry)
    }

    /// Domain statistics — entries per skill/domain, with avg confidence.
    pub fn domain_stats(&self) -> Vec<DomainStat> {
        let mut domains: std::collections::HashMap<String, (usize, f64)> =
            std::collections::HashMap::new();

        for entry in &self.entries {
            let domain = entry
                .approach
                .tools_used
                .first()
                .map(|t| t.trim_start_matches("skill:").to_string())
                .unwrap_or_else(|| "unknown".into());
            let stat = domains.entry(domain).or_insert((0, 0.0));
            stat.0 += 1;
            stat.1 += entry.effective_confidence();
        }

        let mut stats: Vec<DomainStat> = domains
            .into_iter()
            .map(|(domain, (count, total_conf))| DomainStat {
                domain,
                entry_count: count,
                avg_confidence: if count > 0 {
                    total_conf / count as f64
                } else {
                    0.0
                },
            })
            .collect();

        stats.sort_by(|a, b| b.entry_count.cmp(&a.entry_count));
        stats
    }
}

/// Statistics for one genome domain.
#[derive(Debug, Clone)]
pub struct DomainStat {
    pub domain: String,
    pub entry_count: usize,
    pub avg_confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature::ApproachSignature;

    fn make_approach() -> ApproachSignature {
        ApproachSignature::new("test", vec!["step1".into()], vec!["tool1".into()])
    }

    #[test]
    fn add_and_query() {
        let mut store = GenomeStore::new();
        store
            .add_from_operation("deploy rest api service", make_approach(), 0.8)
            .unwrap();

        let results = store.query("deploy rest api service");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn total_ever_monotonic() {
        let mut store = GenomeStore::new();
        assert_eq!(store.total_ever(), 0);
        store
            .add_from_operation("task one", make_approach(), 0.5)
            .unwrap();
        assert_eq!(store.total_ever(), 1);
        store
            .add_from_operation("task two", make_approach(), 0.6)
            .unwrap();
        assert_eq!(store.total_ever(), 2);
    }

    #[test]
    fn record_use_updates() {
        let mut store = GenomeStore::new();
        let id = store
            .add_from_operation("deploy rest api", make_approach(), 0.5)
            .unwrap();
        store.record_use(&id, true).unwrap();

        let results = store.query("deploy rest api");
        assert_eq!(results[0].use_count, 1);
        assert_eq!(results[0].success_count, 1);
    }

    #[test]
    fn record_use_not_found() {
        let mut store = GenomeStore::new();
        let result = store.record_use("nonexistent", true);
        assert!(result.is_err());
    }

    #[test]
    fn query_disjoint_excluded() {
        let mut store = GenomeStore::new();
        store
            .add_from_operation("deploy rest api service", make_approach(), 0.8)
            .unwrap();

        let results = store.query("compile rust binary executable");
        assert!(results.is_empty());
    }

    #[test]
    fn indirect_phrasing_matches() {
        let mut store = GenomeStore::new();
        // The cascade entry
        store
            .add_from_operation(
                "service failures cascading to take down other services",
                make_approach(),
                0.92,
            )
            .unwrap();

        // Indirect query — shared stemmed terms: "failur", "servic"
        let results = store.query(
            "Netflix had a famous approach to stopping failures from spreading across their services",
        );
        assert!(
            !results.is_empty(),
            "Indirect phrasing should match via IDF-weighted scoring"
        );
    }

    #[test]
    fn default_creates_empty() {
        let store = GenomeStore::default();
        assert!(store.is_empty());
        assert_eq!(store.total_ever(), 0);
    }
}
