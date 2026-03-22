//! perceive.rs — Runs the full Layer 2 comprehension pipeline.
//! Every step has a graceful fallback. This never panics.

use hydra_attention::{AttentionEngine, AttentionFrame};
use hydra_comprehension::{ComprehendedInput, ComprehensionEngine, InputSource};
use hydra_context::{AnomalyContext, ContextFrame, GapContext, SessionHistory};
use hydra_genome::GenomeStore;
use hydra_language::LanguageEngine;
use hydra_noticing::NoticingEngine;

use crate::loop_::types::PerceivedInput;

pub struct Perceiver {
    noticing: NoticingEngine,
}

impl Perceiver {
    pub fn new() -> Self {
        Self {
            noticing: NoticingEngine::new(),
        }
    }

    /// Run full Layer 2 pipeline. Always returns a PerceivedInput.
    /// If any stage fails, it degrades gracefully.
    pub fn perceive(&mut self, raw: &str, genome: &GenomeStore) -> PerceivedInput {
        // Stage 1: Comprehension
        let engine = ComprehensionEngine::new();
        let comprehended = match engine.comprehend(raw, InputSource::PrincipalText, genome) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!("comprehension failed: {:?}", e);
                return self.minimal_perceived(raw);
            }
        };

        // Stage 2: Language analysis (stateless — no constructor needed)
        let language = match LanguageEngine::analyze(&comprehended) {
            Ok(l) => Some(l),
            Err(e) => {
                tracing::debug!("language analysis failed: {:?}", e);
                None
            }
        };

        // Stage 3: Context frame (fresh session — no history)
        let history = SessionHistory::new();
        let gap_ctx = GapContext::new();
        let anomaly = AnomalyContext::new();
        let context = ContextFrame::build(&comprehended, &history, &[], &gap_ctx, &anomaly);

        // Stage 4: Attention allocation
        let attention = if context.total_items() > 0 {
            if let Some(ref lang) = language {
                match AttentionEngine::allocate(&comprehended, &context, lang) {
                    Ok(a) => a,
                    Err(e) => {
                        tracing::debug!("attention failed: {:?}", e);
                        AttentionFrame::minimal()
                    }
                }
            } else {
                AttentionFrame::minimal()
            }
        } else {
            AttentionFrame::minimal()
        };

        // Stage 5: Noticing cycle (ambient — always runs)
        let signals: Vec<_> = self
            .noticing
            .cycle()
            .into_iter()
            .filter(|s| s.is_significant())
            .cloned()
            .collect();

        PerceivedInput {
            raw: raw.to_string(),
            comprehended,
            language,
            context,
            attention,
            signals,
            enrichments: std::collections::HashMap::new(),
        }
    }

    fn minimal_perceived(&mut self, raw: &str) -> PerceivedInput {
        let comprehended = ComprehendedInput::minimal(raw);
        let history = SessionHistory::new();
        let gap_ctx = GapContext::new();
        let anomaly = AnomalyContext::new();
        let context = ContextFrame::build(&comprehended, &history, &[], &gap_ctx, &anomaly);
        let attention = AttentionFrame::minimal();
        let signals: Vec<_> = self
            .noticing
            .cycle()
            .into_iter()
            .filter(|s| s.is_significant())
            .cloned()
            .collect();

        PerceivedInput {
            raw: raw.to_string(),
            comprehended,
            language: None,
            context,
            attention,
            signals,
            enrichments: std::collections::HashMap::new(),
        }
    }
}

impl Default for Perceiver {
    fn default() -> Self {
        Self::new()
    }
}
