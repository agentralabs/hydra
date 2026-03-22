//! HydraMemoryBridge — connects the Hydra kernel to AgenticMemory.
//! This is the primary interface for all memory operations.
//! Persists to ~/.hydra/data/hydra.amem on every write.

use std::path::PathBuf;

use crate::{
    constants::EMBEDDING_DIMENSION,
    errors::MemoryError,
    layers::MemoryRecord,
    session::SessionManager,
    temporal_bridge::TemporalBridge,
    verbatim::{ContextSnapshot, Surface, VerbatimRecord},
};
use agentic_memory::{AmemReader, AmemWriter, CognitiveEventBuilder, MemoryGraph, WriteEngine};
use hydra_constitution::{ConstitutionChecker, LawCheckContext};
use hydra_temporal::btree::ManifoldCoord;
use hydra_temporal::timestamp::Timestamp;

/// The HydraMemoryBridge — connects the Hydra kernel to AgenticMemory.
/// Persists to disk on every write for cross-process memory continuity.
pub struct HydraMemoryBridge {
    write_engine: WriteEngine,
    graph: MemoryGraph,
    pub temporal: TemporalBridge,
    pub sessions: SessionManager,
    total_written: u64,
    amem_path: PathBuf,
    checker: ConstitutionChecker,
}

impl HydraMemoryBridge {
    /// Initialize the bridge, loading from disk if a .amem file exists.
    pub fn new() -> Self {
        let amem_path = amem_file_path();

        // Try loading existing graph from disk
        let (graph, total_written) = if amem_path.exists() {
            match AmemReader::read_from_file(&amem_path) {
                Ok(g) => {
                    let count = g.node_count() as u64;
                    eprintln!(
                        "hydra: memory loaded {} nodes from {}",
                        count,
                        amem_path.display()
                    );
                    (g, count)
                }
                Err(e) => {
                    eprintln!("hydra: memory load failed (starting fresh): {e}");
                    (MemoryGraph::new(EMBEDDING_DIMENSION), 0)
                }
            }
        } else {
            (MemoryGraph::new(EMBEDDING_DIMENSION), 0)
        };

        Self {
            write_engine: WriteEngine::new(EMBEDDING_DIMENSION),
            graph,
            temporal: TemporalBridge::new(),
            sessions: SessionManager::new(),
            total_written,
            amem_path,
            checker: ConstitutionChecker::new(),
        }
    }

    /// WRITE-AHEAD: Store a verbatim record before Hydra responds.
    pub fn write_verbatim_ahead(
        &mut self,
        input: impl Into<String>,
        surface: Surface,
        context: ContextSnapshot,
        causal_root: impl Into<String>,
    ) -> Result<VerbatimRecord, MemoryError> {
        let causal_root_str = causal_root.into();
        let sequence = self.sessions.current.exchange_count;

        let record = VerbatimRecord::begin(
            self.sessions.session_id().to_string(),
            sequence,
            surface,
            input,
            context,
            causal_root_str.clone(),
        )?;

        let mem_record = record.to_memory_record();
        self.write_memory_record(&mem_record, &causal_root_str)?;

        self.sessions.record_exchange();
        self.total_written += 1;

        Ok(record)
    }

    /// Finalize a verbatim record after Hydra has responded.
    pub fn finalize_verbatim(
        &mut self,
        mut record: VerbatimRecord,
        response: impl Into<String>,
        manifold_after: f64,
        causal_root: &str,
    ) -> Result<(), MemoryError> {
        record.finalize(response, manifold_after);
        record.verify_integrity()?;

        let mem_record = record.to_memory_record();
        self.write_memory_record(&mem_record, causal_root)?;

        Ok(())
    }

    /// Write any memory record to AgenticMemory and persist to disk.
    /// Constitutional Law 3 (Memory Sovereignty) is enforced here.
    pub fn write_memory_record(
        &mut self,
        record: &MemoryRecord,
        causal_root: &str,
    ) -> Result<(), MemoryError> {
        // Constitutional check: Law 3 (Memory Sovereignty)
        let ctx = LawCheckContext::new(&record.session_id, "memory.write")
            .with_meta("layer", record.layer.tag())
            .with_meta("causal_root", causal_root);
        if let Err(e) = self.checker.check_strict(&ctx) {
            eprintln!("hydra: memory write BLOCKED by constitution: {e}");
            return Err(MemoryError::WriteError {
                reason: format!("constitutional violation: {e}"),
            });
        }

        let content = record.to_cognitive_content();
        let event_type = record.layer.event_type();
        let ts = Timestamp::now();

        let event = CognitiveEventBuilder::new(event_type, content).build();

        let result = self
            .write_engine
            .ingest(&mut self.graph, vec![event], vec![])
            .map_err(|e| MemoryError::WriteError {
                reason: e.to_string(),
            })?;

        if let Some(&node_id) = result.new_node_ids.first() {
            let memory_id = node_id.to_string();
            self.temporal.index(
                &memory_id,
                ts,
                ManifoldCoord::new(0.0, 0.0, 0.0),
                causal_root,
                &record.session_id,
            )?;
        }

        // Persist to disk after every write — propagate error so caller knows
        if let Err(e) = self.persist_to_disk() {
            eprintln!("hydra: memory persist failed: {e}");
            return Err(e);
        }

        Ok(())
    }

    /// Persist the current graph to the .amem file.
    /// Returns error if disk write fails so the caller knows persistence state.
    fn persist_to_disk(&self) -> Result<(), MemoryError> {
        if let Some(parent) = self.amem_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| MemoryError::WriteError {
                    reason: format!("mkdir failed: {e}"),
                })?;
            }
        }
        let writer = AmemWriter::new(EMBEDDING_DIMENSION);
        writer
            .write_to_file(&self.graph, &self.amem_path)
            .map_err(|e| MemoryError::WriteError {
                reason: format!("persist to disk failed: {e}"),
            })
    }

    /// Query: most recent N memories.
    pub fn recent(&self, n: usize) -> Vec<String> {
        self.temporal
            .most_recent(n)
            .iter()
            .map(|e| e.memory_id.to_string())
            .collect()
    }

    /// Query: retrieve content of a node by ID.
    pub fn get_content(&self, node_id: u64) -> Option<String> {
        self.graph.get_node(node_id).map(|n| n.content.clone())
    }

    /// Query: all recent node contents (for prompt enrichment).
    pub fn recent_contents(&self, n: usize) -> Vec<String> {
        let total = self.graph.node_count();
        if total == 0 {
            return Vec::new();
        }
        let start = total.saturating_sub(n);
        self.graph
            .nodes()
            .iter()
            .skip(start)
            .map(|n| n.content.clone())
            .collect()
    }

    /// IDF-scored memory retrieval — returns the most RELEVANT nodes
    /// for the given query, not just the most recent.
    ///
    /// Math: score(node) = Σ IDF(term) × recency_weight
    /// where IDF(term) = ln((N+1) / (df(term)+1))
    /// and recency_weight = 1.0 for newest, decays toward 0.3 for oldest.
    ///
    /// Then deduplicates by topic: if two nodes share >60% of terms,
    /// only the higher-scored one is kept. This prevents 5 circuit-breaker
    /// exchanges from flooding out all other topics.
    pub fn query_relevant(&self, query: &str, max_results: usize) -> Vec<String> {
        let nodes = self.graph.nodes();
        let total = nodes.len();
        if total == 0 {
            return Vec::new();
        }

        // Stem and tokenize query
        let query_terms: Vec<String> = query
            .split_whitespace()
            .map(|w| w.to_lowercase().replace(|c: char| !c.is_alphanumeric(), ""))
            .filter(|w| w.len() >= 3)
            .collect();

        if query_terms.is_empty() {
            return self.recent_contents(max_results);
        }

        // Compute document frequency for each query term
        let n = total as f64;
        let idfs: Vec<(String, f64)> = query_terms
            .iter()
            .map(|term| {
                let df = nodes
                    .iter()
                    .filter(|node| node.content.to_lowercase().contains(term.as_str()))
                    .count() as f64;
                let idf = ((n + 1.0) / (df + 1.0)).ln();
                (term.clone(), idf)
            })
            .collect();

        // Score each node: IDF × recency × relevance bonus
        // Key insight: if IDF score > 2.0 (highly relevant), override
        // temporal decay. A circuit breaker discussion from yesterday
        // is more useful than a generic exchange from 5 minutes ago.
        // This is how human memory works — you remember what matters.
        let mut scored: Vec<(f64, usize, &str)> = nodes
            .iter()
            .enumerate()
            .map(|(idx, node)| {
                let content_lower = node.content.to_lowercase();
                let idf_score: f64 = idfs
                    .iter()
                    .filter(|(term, _)| content_lower.contains(term.as_str()))
                    .map(|(_, idf)| idf)
                    .sum();

                // Recency: newest=1.0, oldest=0.3
                let recency = 0.3 + 0.7 * (idx as f64 / n.max(1.0));

                // Recency bonus for very recent nodes
                let age_fraction = 1.0 - (idx as f64 / n.max(1.0));
                let recency_bonus = if age_fraction < 0.1 { 0.5 }    // last 10%
                    else if age_fraction < 0.3 { 0.2 }                // last 30%
                    else { 0.0 };

                // Relevance override: if highly relevant, age doesn't matter
                let score = if idf_score > 2.0 {
                    // Strong relevance — override decay, keep full IDF
                    idf_score * (1.0 + recency_bonus)
                } else {
                    // Normal — apply temporal decay
                    idf_score * recency * (1.0 + recency_bonus)
                };

                (score, idx, node.content.as_str())
            })
            .filter(|(score, _, _)| *score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Deduplicate by topic: skip nodes that share >60% terms with a kept node
        let mut kept: Vec<String> = Vec::new();
        let mut kept_terms: Vec<Vec<String>> = Vec::new();

        for (_, _, content) in &scored {
            if kept.len() >= max_results {
                break;
            }
            let terms: Vec<String> = content
                .split_whitespace()
                .take(20)
                .map(|w| w.to_lowercase())
                .collect();

            // Check overlap with already-kept nodes
            let is_duplicate = kept_terms.iter().any(|existing| {
                if existing.is_empty() || terms.is_empty() {
                    return false;
                }
                let overlap = terms.iter().filter(|t| existing.contains(t)).count();
                let max_len = existing.len().max(terms.len());
                (overlap as f64 / max_len as f64) > 0.6
            });

            if !is_duplicate {
                kept.push(content.to_string());
                kept_terms.push(terms);
            }
        }

        // If IDF found nothing, fall back to recent
        if kept.is_empty() {
            return self.recent_contents(max_results);
        }

        kept
    }

    pub fn at_timestamp(&self, ts: &Timestamp) -> Option<String> {
        self.temporal.get_exact(ts).map(|e| e.memory_id.to_string())
    }

    pub fn total_written(&self) -> u64 {
        self.total_written
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn session_id(&self) -> &str {
        self.sessions.session_id()
    }

    pub fn health(&self) -> MemoryHealth {
        MemoryHealth {
            total_written: self.total_written,
            temporal_indexed: self.temporal.total_indexed(),
            session_id: self.sessions.session_id().to_string(),
            exchange_count: self.sessions.current.exchange_count,
            persistent_nodes: self.graph.node_count() as u64,
        }
    }
}

impl Default for HydraMemoryBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory health summary for TUI display.
#[derive(Debug, Clone)]
pub struct MemoryHealth {
    pub total_written: u64,
    pub temporal_indexed: u64,
    pub session_id: String,
    pub exchange_count: u64,
    pub persistent_nodes: u64,
}

/// Returns ~/.hydra/data/hydra.amem
fn amem_file_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hydra")
        .join("data")
        .join("hydra.amem")
}
