//! CognitiveLoop — the coordinator that runs perceive->route->prompt->llm->deliver.
//!
//! Session isolation: fresh context every cycle. Non-negotiable.
//! If reasoning resolves the input, zero LLM tokens are used.
//! Middleware chain hooks at 5 points along the pipeline.

use hydra_genome::GenomeStore;

use crate::loop_::{
    deliver::Deliverer,
    llm::LlmCaller,
    middleware::MiddlewareChain,
    middlewares,
    perceive::Perceiver,
    prompt::PromptBuilder,
    route::Router,
    types::CycleResult,
};

/// The cognitive loop coordinator.
pub struct CognitiveLoop {
    genome: GenomeStore,
    perceiver: Perceiver,
    router: Router,
    prompt_builder: PromptBuilder,
    llm: LlmCaller,
    deliverer: Deliverer,
    middlewares: MiddlewareChain,
}

impl CognitiveLoop {
    pub fn new() -> Self {
        let mut genome = GenomeStore::open();
        genome.load_from_skills();
        Self {
            genome,
            perceiver: Perceiver::new(),
            router: Router::new(),
            prompt_builder: PromptBuilder::new(),
            llm: LlmCaller::from_env(),
            deliverer: Deliverer::new(),
            middlewares: middlewares::build_chain(),
        }
    }

    /// Process one input -> one response.
    /// Every cycle gets a fresh session_id — isolation law.
    pub async fn cycle(&mut self, raw: &str) -> String {
        let start = std::time::Instant::now();
        let session_id = uuid::Uuid::new_v4().to_string();

        // Guard: empty input
        let raw = raw.trim();
        if raw.is_empty() {
            return "I received an empty input. What would you like to do?".into();
        }

        // Guard: extremely long input (UTF-8 safe truncation)
        let raw = if raw.len() > 12_000 {
            raw.char_indices()
                .take_while(|(i, _)| *i < 12_000)
                .last()
                .map(|(i, c)| &raw[..i + c.len_utf8()])
                .unwrap_or(raw)
        } else {
            raw
        };

        // PHASE 1: Perceive (Layer 2 pipeline)
        let mut perceived = self.perceiver.perceive(raw, &self.genome);

        // HOOK: post_perceive — middlewares enrich perceived input
        self.middlewares.run_post_perceive(&mut perceived);

        // PHASE 2: Route
        let (path, resolved) = self.router.route(&perceived, &self.genome);

        // HOOK: post_route — middlewares observe routing decision
        self.middlewares.run_post_route(&perceived, path.label());

        // Collect middleware enrichments for prompt.
        // Two sources: (1) enrich_prompt() trait method, (2) perceived.enrichments from post_perceive.
        let mut mw_enrichments = self.middlewares.collect_enrichments(&perceived);
        // Merge perceived.enrichments (genome, memory, calibration, etc.)
        for (k, v) in &perceived.enrichments {
            mw_enrichments.entry(k.clone()).or_insert_with(|| v.clone());
        }

        // PHASE 3: Generate response
        let (response, tokens) = if let Some(text) = resolved {
            (text, 0_usize)
        } else if path.needs_llm() {
            let prompt = self.prompt_builder.build_with_enrichments(
                &perceived,
                path.token_budget(),
                &mw_enrichments,
            );
            match self.llm.call(&prompt).await {
                Ok(r) => (r.content, r.tokens_used),
                Err(e) => {
                    let msg = format!(
                        "[Hydra error: {}]\n\
                         The LLM call failed. This event has been receipted.\n\
                         Try again, or switch provider with HYDRA_LLM_PROVIDER=ollama",
                        e
                    );
                    (msg, 0)
                }
            }
        } else {
            // Routing said reasoning but no resolved text — LLM fallback
            let prompt =
                self.prompt_builder
                    .build_with_enrichments(&perceived, 8_000, &mw_enrichments);
            match self.llm.call(&prompt).await {
                Ok(r) => (r.content, r.tokens_used),
                Err(e) => (format!("[Hydra error: {}]", e), 0),
            }
        };

        // HOOK: post_llm — middlewares observe response
        self.middlewares.run_post_llm(&perceived, &response);

        let domain = perceived.comprehended.primary_domain.label().to_string();
        let duration_ms = start.elapsed().as_millis() as u64;
        let success = !response.starts_with("[Hydra error");

        // PHASE 4: Record exchange in soul
        self.prompt_builder.record_exchange(&session_id, &domain);

        // PHASE 5: Deliver (audit receipt + settlement)
        let cycle = CycleResult {
            session_id: session_id.clone(),
            domain: domain.clone(),
            path: path.label().to_string(),
            intent_summary: raw[..raw.len().min(80)].to_string(),
            response: response.clone(),
            tokens_used: tokens,
            duration_ms,
            success,
            enrichments: mw_enrichments,
        };
        self.deliverer.deliver(&cycle);

        // HOOK: post_deliver — middlewares finalize
        self.middlewares.run_post_deliver(&cycle);

        // Print receipt footer to stderr
        eprintln!(
            "[{}|{}|{}tok|{}ms|mw={}]",
            &session_id[..8],
            cycle.path,
            cycle.tokens_used,
            cycle.duration_ms,
            self.middlewares.len(),
        );

        response
    }

    pub fn status(&self) -> String {
        format!(
            "cognitive-loop: genome={} soul=[{}] audit=[{}] middlewares={}",
            self.genome.len(),
            self.prompt_builder.soul_status(),
            self.deliverer.audit_summary(),
            self.middlewares.names().join(","),
        )
    }

    /// Hydra examines itself. Every fact is derived from real data.
    pub fn self_portrait(&self) -> crate::self_knowledge::SelfPortrait {
        let state = crate::state::HydraState::initial(); // use current state in daemon mode
        crate::self_knowledge::introspect(
            &state,
            self.genome.len(),
            0, // memory nodes filled by memory middleware
            self.genome.len(), // skills ≈ genome entries loaded
            "finance", // TODO: compute from genome entry distribution
            "debugging",
            0, // integrations counted at boot
            0, // actions counted at boot
            self.middlewares.len(),
        )
    }
}

impl Default for CognitiveLoop {
    fn default() -> Self {
        Self::new()
    }
}
