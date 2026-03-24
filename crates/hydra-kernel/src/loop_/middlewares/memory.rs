//! Memory middleware — hydra-memory per-request.
//!
//! ⚠️  SACRED FILE — READ MEMORY_SACRED.md BEFORE MODIFYING ⚠️
//!
//! Contains EMI, NEC, and session-bounded evidence templates that took
//! memory scores from 1.7/10 to 9.0/10. Every word is calibrated through
//! 6+ harness runs. Changing a single sentence can drop scores by 6 points.
//! See MEMORY_SACRED.md in this directory for the full protection protocol.

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
        let relevant = if node_count > 0 {
            self.bridge.query_relevant(&perceived.raw, 8)
        } else {
            Vec::new()
        };
        let summaries: Vec<String> = relevant
            .iter()
            .filter_map(|raw| extract_exchange_summary(raw))
            .collect();

        if !summaries.is_empty() {
            // EMI (Evidential Memory Injection) — closed-world with evidence
            let total = summaries.len();
            let mut evidence = format!(
                "MEMORY EVIDENCE (closed world — no facts exist beyond this list):\n\
                 TOTAL EVIDENCE ITEMS: {total}\n\
                 CRITICAL DISTINCTION:\n\
                 - These items are from PRIOR sessions with UNKNOWN users, NOT from this conversation.\n\
                 - You do NOT know this user personally. You have NOT spoken to them before.\n\
                 - You may use these items as GENERAL KNOWLEDGE you have encountered, NOT as personal history with this user.\n\
                 - If asked 'what have WE discussed' or 'what patterns do you see in MY questions' or 'what kind of problems do I bring': \
                   the honest answer is 'this is our first conversation' or 'I don't have enough history with you yet.'\n\
                 - NEVER say 'based on our previous conversations' or 'you tend to ask about' — that is fabrication.\n\
                 EVIDENCE:\n"
            );
            for (i, summary) in summaries.iter().enumerate() {
                let age_note = memory_age_note(i, node_count);
                evidence.push_str(&format!("  [{}] {}{}\n", i + 1, summary, age_note));
            }
            evidence.push_str("END OF EVIDENCE.\n");
            evidence.push_str("RULES:\n");
            evidence.push_str(&format!(
                "  - You may reference items [1]-[{total}] as general knowledge.\n\
                 - You may NOT present them as personal history with this user.\n\
                 - If asked about a topic not in [1]-[{total}], say \"I don't have that in memory.\"\n\
                 - The number {total} is the TOTAL. Not more. Exactly {total}."
            ));
            perceived.enrichments.insert("memory.context".into(), evidence);
        } else {
            // NEC (Null Evidence Certificate) — closed-world with ZERO evidence
            // This is critical: without this, the LLM fabricates history.
            // The certificate proves memory was queried and returned nothing.
            let nec = format!(
                "MEMORY EVIDENCE (closed world — NULL CERTIFICATE):\n\
                 TOTAL EVIDENCE ITEMS: 0\n\
                 Memory store queried: {} nodes searched, 0 relevant results.\n\
                 NULL CERTIFICATE: This is the first interaction OR no prior exchanges are relevant.\n\
                 RULES:\n\
                 - You have NO prior conversation history with this user.\n\
                 - Do NOT fabricate, invent, or assume any prior exchanges.\n\
                 - If asked about prior conversations, say: \"We haven't discussed that yet.\"\n\
                 - If asked about patterns in the user's questions, say: \"This is our first exchange\" or \"I don't have enough history yet.\"\n\
                 - Fabricating memory is a CONSTITUTIONAL VIOLATION. Honesty is mandatory.",
                node_count,
            );
            perceived.enrichments.insert("memory.context".into(), nec);
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

/// Generate an age annotation for a memory evidence item.
/// Older items (higher index relative to total) get age notes.
fn memory_age_note(index: usize, total_nodes: usize) -> String {
    if total_nodes < 10 {
        return String::new(); // Not enough history to judge age
    }
    // IDF already handles relevance; this adds temporal context
    let recency_ratio = if total_nodes > 0 {
        1.0 - (index as f64 / 8.0) // rough: first items are most recent/relevant
    } else {
        1.0
    };
    if recency_ratio < 0.3 {
        " (older memory)".into()
    } else {
        String::new()
    }
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
