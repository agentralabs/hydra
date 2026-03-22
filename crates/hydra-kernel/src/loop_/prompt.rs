//! prompt.rs — Build enriched LLM prompt from perceived input.
//!
//! Prompt structure follows attention distribution research:
//! - Tier 1 (identity): beginning of system prompt = highest attention
//! - Tier 2 (memory): immediately after identity = high attention (recency)
//! - Tier 3 (genome): imperative instructions = high compliance
//! - Tier 4 (soul/patterns): context enrichment
//! - Tier 5 (subsystem insights): end of system prompt = second-highest attention (primacy)

use hydra_pattern::PatternEngine;
use hydra_soul::{NodeKind, Soul};

use crate::loop_::types::PerceivedInput;

/// The assembled prompt ready for LLM submission.
pub struct EnrichedPrompt {
    pub system: String,
    pub user: String,
    pub budget: usize,
}

pub struct PromptBuilder {
    soul: Soul,
    patterns: PatternEngine,
}

impl PromptBuilder {
    pub fn new() -> Self {
        Self {
            soul: Soul::new(),
            patterns: PatternEngine::new(),
        }
    }

    pub fn build_with_enrichments(
        &self,
        perceived: &PerceivedInput,
        budget: usize,
        mw_enrichments: &std::collections::HashMap<String, String>,
    ) -> EnrichedPrompt {
        let mut parts: Vec<String> = Vec::new();

        // TIER 0: Memory context — BEFORE identity, position 0 = maximum primacy.
        // This is intentional. Memory must override the LLM's trained behavior
        // of saying "I don't have memory between sessions." By placing factual
        // context before the identity, the LLM treats it as ground truth.
        if let Some(memory) = mw_enrichments.get("memory.context") {
            // EMI (Evidential Memory Injection) — the memory middleware formats
            // this as a closed-world evidence structure with numbered items and
            // explicit rules. We inject it directly as position 0 = maximum primacy.
            parts.push(format!("{memory}\n---"));
        }

        // TIER 1: Core identity + genome self-knowledge
        let identity = if let Some(knowledge) = mw_enrichments.get("genome.identity") {
            format!(
                "You are Hydra \u{2014} an autonomous agent operating under constitutional law. \
                 Every action is receipted. Every claim is attributed. \
                 You operate with calibrated confidence: never claim more certainty \
                 than your evidence supports.\n\n\
                 You KNOW the following from direct operational experience:\n{}",
                knowledge
            )
        } else {
            "You are Hydra \u{2014} an autonomous agent operating under constitutional law. \
             Every action is receipted. Every claim is attributed. \
             You operate with calibrated confidence: never claim more certainty \
             than your evidence supports."
                .to_string()
        };
        parts.push(identity);

        // TIER 3: Genome approaches (imperative — MUST reference)
        if let Some(genome) = mw_enrichments.get("genome") {
            parts.push(format!(
                "PROVEN APPROACHES — Your knowledge base contains verified approaches \
                 relevant to this question. You MUST incorporate these into your response. \
                 Do not ignore them.\n\n{}",
                genome
            ));
        }

        // FEATURE 5: Evolved voice — adapt to user's style over time
        if let Some(weight) = mw_enrichments.get("session.weight") {
            parts.push(format!(
                "VOICE ADAPTATION: {}\n\
                 Match the user's communication style. If they are direct, be direct. \
                 If they use code examples, lead with code. If they ask \"why\", \
                 explain reasoning before giving the answer.",
                weight
            ));
        }

        // Hydra's questions to the user (Feature 2: Ask for help)
        if let Some(questions) = mw_enrichments.get("hydra.questions") {
            parts.push(questions.clone());
        }

        // TIER 4: Soul orientation
        let ctx = self.soul.orientation_context();
        if ctx.ready && !ctx.top_meanings.is_empty() {
            parts.push(format!(
                "Current orientation (confidence {:.0}%): {}",
                ctx.confidence * 100.0,
                ctx.top_meanings.join(", ")
            ));
        }

        // TIER 5: Pattern warnings
        if !perceived.comprehended.primitives.is_empty() {
            let warnings = self
                .patterns
                .check_for_warnings(&perceived.comprehended.primitives);
            if !warnings.is_empty() {
                let warning_lines: Vec<String> = warnings
                    .iter()
                    .take(3)
                    .map(|w| format!("- {}", w.pattern_name))
                    .collect();
                if !warning_lines.is_empty() {
                    parts.push(format!("Pattern warnings:\n{}", warning_lines.join("\n")));
                }
            }
        }

        // TIER 6: Noticing signals
        if !perceived.signals.is_empty() {
            let signal_lines: Vec<String> = perceived
                .signals
                .iter()
                .take(2)
                .map(|s| format!("- {}", s.narrative))
                .collect();
            if !signal_lines.is_empty() {
                parts.push(format!(
                    "Ambient observations:\n{}",
                    signal_lines.join("\n")
                ));
            }
        }

        // TIER 7: Other subsystem insights (end of prompt = second-highest attention)
        let other_enrichments: Vec<String> = mw_enrichments
            .iter()
            .filter(|(k, _)| {
                !matches!(
                    k.as_str(),
                    "memory.context"
                        | "genome"
                        | "genome.identity"
                        | "session.weight"
                        | "hydra.questions"
                )
            })
            .map(|(name, content)| format!("[{name}] {content}"))
            .collect();
        if !other_enrichments.is_empty() {
            parts.push(format!(
                "Subsystem analysis:\n{}",
                other_enrichments.join("\n")
            ));
        }

        let system = parts.join("\n\n");

        // Enforce budget
        let max_user_chars = (budget * 4).saturating_sub(system.len()).max(100);
        let user = if perceived.raw.len() > max_user_chars {
            format!(
                "{}...[truncated to fit {}-token budget]",
                &perceived.raw[..max_user_chars],
                budget
            )
        } else {
            perceived.raw.clone()
        };

        EnrichedPrompt {
            system,
            user,
            budget,
        }
    }

    pub fn build(&self, perceived: &PerceivedInput, budget: usize) -> EnrichedPrompt {
        self.build_with_enrichments(perceived, budget, &std::collections::HashMap::new())
    }

    pub fn record_exchange(&mut self, session_id: &str, domain: &str) {
        let kind = if domain != "unknown" {
            NodeKind::RecurringChoice
        } else {
            NodeKind::RecurringReturn
        };
        if let Err(e) = self.soul.record_exchange(session_id, kind) {
            tracing::debug!("soul record_exchange failed: {:?}", e);
        }
    }

    pub fn soul_status(&self) -> String {
        self.soul.status_line()
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}
