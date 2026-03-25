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
    /// Cache: hash of last system prompt to detect changes.
    last_system_hash: std::cell::Cell<Option<u64>>,
    /// Count of times the system prompt was reused (cache hit).
    cache_hits: std::cell::Cell<u64>,
}

impl PromptBuilder {
    pub fn new() -> Self {
        Self {
            soul: Soul::new(),
            patterns: PatternEngine::new(),
            last_system_hash: std::cell::Cell::new(None),
            cache_hits: std::cell::Cell::new(0),
        }
    }

    pub fn build_with_enrichments(
        &self,
        perceived: &PerceivedInput,
        budget: usize,
        mw_enrichments: &std::collections::HashMap<String, String>,
    ) -> EnrichedPrompt {
        // SEC-3: Redact credentials from all enrichments before they reach the LLM
        let mut safe_enrichments = mw_enrichments.clone();
        redact_credentials_in_enrichments(&mut safe_enrichments);
        let mw_enrichments = &safe_enrichments;
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

        // TIER 0.5: HEFP — Epistemic Calibration Protocol (binding constraint)
        if let Some(hefp) = mw_enrichments.get("calibration.hefp") {
            parts.push(format!(
                "EPISTEMIC CALIBRATION PROTOCOL (binding):\n{hefp}\n\
                 Rules:\n\
                 1. If WELL-CALIBRATED: express strong confidence. Say you are confident, certain, and familiar.\n\
                 2. If LIMITED DATA: express moderate confidence with honest hedging.\n\
                 3. If NO DATA: do not claim specific percentages. Hedge appropriately.\n\
                 4. If STOCHASTIC: state that prediction is inherently limited.\n\
                 5. When asked about confidence, cite the methodology (Beta posterior, observations, CI).\n\
                 6. Never fabricate confidence percentages — only cite numbers from this protocol.\n\
                 ---"
            ));
        }

        // Language matching: respond in whatever language the user writes
        if let Some(lang) = mw_enrichments.get("detected_language") {
            if lang != "english" && lang != "en" {
                parts.push(format!(
                    "The user is writing in {lang}. You MUST respond in {lang}. \
                     Match their language exactly. Do not switch to English unless they do."
                ));
            }
        }

        // TIER 1: Core identity + genome self-knowledge
        let identity = if let Some(knowledge) = mw_enrichments.get("genome.identity") {
            format!(
                "You are Hydra \u{2014} an autonomous agent. \
                 Be conversational and helpful. Do not output status reports, receipts, \
                 operational metadata, or audit trails in your responses. \
                 Just answer the user naturally.\n\n\
                 You KNOW the following from direct operational experience:\n{}",
                knowledge
            )
        } else {
            "You are Hydra \u{2014} an autonomous agent operating under constitutional law. \
             Every action is receipted. Every claim is attributed. \
             You operate with calibrated confidence: never claim more certainty \
             than your evidence supports. \
             You can create skills: users drop markdown into ~/.hydra/drop/ or use /learn <file>. \
             You self-evolve by detecting gaps and generating new capabilities. \
             Credentials you create or receive MUST be stored in the vault — protect them absolutely. \
             If you open an account or generate a key, self-drop it to ~/.hydra/drop/ for vault storage."
                .to_string()
        };
        parts.push(identity);

        // TIER 3: Genome approaches (imperative — MUST reference)
        if let Some(genome) = mw_enrichments.get("genome") {
            parts.push(format!(
                "PROVEN APPROACHES — Your knowledge base contains verified approaches \
                 relevant to this question. You MUST incorporate these into your response. \
                 When citing an approach, include its confidence and observations: (conf=X% obs=N strength=LEVEL).\n\n{}",
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

        // Dream insights — what Hydra learned in background
        if let Some(dream) = mw_enrichments.get("dream.insights") {
            parts.push(format!("RECENT INSIGHTS (background reasoning):\n{dream}\nApply if relevant."));
        }
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
                        | "calibration.hefp"
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

        // Prompt cache: track if system prompt changed since last cycle.
        // When it hasn't changed, the LLM provider can use its prompt cache
        // (Anthropic caches identical system prompts, reducing cost).
        let system_hash = Self::hash_string(&system);
        if self.last_system_hash.get() == Some(system_hash) {
            self.cache_hits.set(self.cache_hits.get() + 1);
        }
        self.last_system_hash.set(Some(system_hash));

        EnrichedPrompt {
            system,
            user,
            budget,
        }
    }

    /// Simple hash for cache comparison (not crypto, just identity check).
    fn hash_string(s: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// How many times the system prompt was identical to the previous cycle.
    pub fn cache_hit_count(&self) -> u64 {
        self.cache_hits.get()
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
    fn default() -> Self { Self::new() }
}

/// SEC-3: Redact credential patterns from all enrichment values before LLM sees them.
fn redact_credentials_in_enrichments(enrichments: &mut std::collections::HashMap<String, String>) {
    let prefixes = ["sk-", "AKIA", "ghp_", "ghs_", "Bearer ", "password=", "token=", "secret="];
    for val in enrichments.values_mut() {
        for p in &prefixes {
            while let Some(pos) = val.find(p) {
                let end = val[pos..].find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                    .map(|e| pos + e).unwrap_or(val.len());
                val.replace_range(pos..end, &format!("{}[REDACTED]", &p[..p.len().min(3)]));
            }
        }
    }
}
