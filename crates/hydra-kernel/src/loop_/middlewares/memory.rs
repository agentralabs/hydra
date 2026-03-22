//! Memory middleware — hydra-memory per-request.
//!
//! Writes verbatim records before response, finalizes after.
//! Injects recent conversation context as human-readable summaries
//! so the LLM can reference prior exchanges naturally.

use hydra_memory::{ContextSnapshot, HydraMemoryBridge, Surface, VerbatimRecord};

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct MemoryMiddleware {
    bridge: HydraMemoryBridge,
    pending_record: Option<VerbatimRecord>,
}

impl MemoryMiddleware {
    pub fn new() -> Self {
        Self {
            bridge: HydraMemoryBridge::new(),
            pending_record: None,
        }
    }
}

impl CycleMiddleware for MemoryMiddleware {
    fn name(&self) -> &'static str {
        "memory"
    }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        // IDF-scored memory retrieval: find nodes RELEVANT to this query,
        // not just the most recent. Then deduplicate by topic so the LLM
        // sees diverse context, not 10 variations of the same exchange.
        let node_count = self.bridge.node_count();
        if node_count > 0 {
            let relevant = self.bridge.query_relevant(&perceived.raw, 8);
            if !relevant.is_empty() {
                let summaries: Vec<String> = relevant
                    .iter()
                    .filter_map(|raw| extract_exchange_summary(raw))
                    .collect();

                if !summaries.is_empty() {
                    // Evidential Memory Injection (EMI) — closed-world format
                    let total = summaries.len();
                    let mut evidence = format!(
                        "MEMORY EVIDENCE (closed world — no facts exist beyond this list):\n\
                         TOTAL EVIDENCE ITEMS: {total}\nEVIDENCE:\n"
                    );
                    for (i, summary) in summaries.iter().enumerate() {
                        evidence.push_str(&format!("  [{}] {}\n", i + 1, summary));
                    }
                    evidence.push_str("END OF EVIDENCE.\n");
                    evidence.push_str("RULES:\n");
                    evidence.push_str(&format!(
                        "  - You may reference items [1]-[{total}] by number.\n\
                         - You may NOT reference any item not listed above.\n\
                         - If asked about a topic not in [1]-[{total}], say \"I don't have that in memory.\"\n\
                         - The number {total} is the TOTAL. Not more. Exactly {total}."
                    ));
                    perceived.enrichments.insert(
                        "memory.context".into(),
                        evidence,
                    );
                }
            }
        }

        // Write-ahead: begin verbatim record before processing
        match self.bridge.write_verbatim_ahead(
            &perceived.raw,
            Surface::Tui,
            ContextSnapshot::default(),
            "cognitive-loop",
        ) {
            Ok(record) => {
                self.pending_record = Some(record);
            }
            Err(e) => {
                eprintln!("hydra: memory post_perceive: {e}");
            }
        }
    }

    fn post_llm(&mut self, _perceived: &PerceivedInput, _response: &str) {}

    fn post_deliver(&mut self, cycle: &CycleResult) {
        if let Some(record) = self.pending_record.take() {
            if let Err(e) = self.bridge.finalize_verbatim(
                record,
                &cycle.response,
                0.0,
                "cognitive-loop",
            ) {
                eprintln!("hydra: memory post_deliver: {e}");
            }
        }

        let total = self.bridge.total_written();
        if total > 0 && total % 100 == 0 {
            eprintln!(
                "hydra: memory milestone: {} records (session={})",
                total,
                self.bridge.session_id()
            );
        }
    }
}

/// Extract a human-readable summary from raw CognitiveEvent content.
/// Actual format: "hydra:verbatim | session:abc | causal:cognitive-loop | <content>"
/// The content after the last " | " is the actual user input or response text.
fn extract_exchange_summary(raw: &str) -> Option<String> {
    // Split on " | " — the actual content is the last segment
    let segments: Vec<&str> = raw.splitn(4, " | ").collect();
    let content = if segments.len() >= 4 {
        segments[3] // actual content after tag/session/causal
    } else if segments.len() >= 2 {
        segments.last().unwrap_or(&"")
    } else {
        raw
    };

    let content = content.trim();
    if content.len() < 5 {
        return None;
    }

    let truncated = if content.len() > 100 {
        format!("{}...", &content[..100])
    } else {
        content.to_string()
    };
    Some(format!("• {truncated}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_middleware_name() {
        let mw = MemoryMiddleware::new();
        assert_eq!(mw.name(), "memory");
    }

    #[test]
    fn extract_summary_from_structured() {
        let raw = "hydra:verbatim | session:abc | causal:cognitive-loop | what is a circuit breaker";
        let summary = extract_exchange_summary(raw);
        assert!(summary.is_some());
        assert!(summary.unwrap().contains("circuit breaker"));
    }

    #[test]
    fn extract_summary_returns_none_for_empty() {
        assert!(extract_exchange_summary("").is_none());
        assert!(extract_exchange_summary("ab").is_none());
    }
}

impl Default for MemoryMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
