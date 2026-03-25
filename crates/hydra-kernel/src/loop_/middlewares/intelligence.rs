//! Intelligence middleware — calibration, wisdom, oracle, omniscience, redteam, genome.
//!
//! Enriches the cognitive loop with judgment, projection, gap detection,
//! adversarial analysis, and genome-proven approaches.
//! All per-request, all non-blocking.

use hydra_calibration::{CalibrationEngine, JudgmentType};
use hydra_genome::GenomeStore;
use hydra_horizon::Horizon;
use hydra_noticing::SurpriseDetector;
use hydra_omniscience::{GapType, OmniscienceEngine};
use hydra_oracle::OracleEngine;
use hydra_redteam::RedTeamEngine;
use hydra_wisdom::{WisdomEngine, WisdomInput};

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct IntelligenceMiddleware {
    calibration: CalibrationEngine,
    wisdom: WisdomEngine,
    oracle: OracleEngine,
    omniscience: OmniscienceEngine,
    redteam: RedTeamEngine,
    genome: GenomeStore,
    surprise: SurpriseDetector,
    horizon: Horizon,
    exchange_count: u64,
    /// Web knowledge index — maps topics to best sources, eliminates blind searches.
    knowledge_index: crate::web_knowledge::KnowledgeIndex,
}

impl IntelligenceMiddleware {
    pub fn new() -> Self {
        let mut genome = GenomeStore::open();
        genome.load_from_skills();

        Self {
            calibration: CalibrationEngine::new(),
            wisdom: WisdomEngine::new(),
            oracle: OracleEngine::new(),
            omniscience: OmniscienceEngine::new(),
            redteam: RedTeamEngine::new(),
            genome,
            surprise: SurpriseDetector::new(),
            horizon: Horizon::new(),
            exchange_count: 0,
            knowledge_index: crate::web_knowledge::KnowledgeIndex::new(),
        }
    }
}

impl CycleMiddleware for IntelligenceMiddleware {
    fn name(&self) -> &'static str {
        "intelligence"
    }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        let domain = perceived.comprehended.primary_domain.label();
        let conf = perceived.comprehended.confidence;

        // Language detection — respond in the user's language
        let detected_lang = detect_language(&perceived.raw);
        if detected_lang != "english" {
            perceived.enrichments.insert("detected_language".into(), detected_lang);
        }

        // GENOME AS IDENTITY: Top genome entries become self-knowledge.
        // Not "advice to follow" — knowledge Hydra HAS from experience.
        // These go into the identity tier of the prompt, always present.
        if !self.genome.is_empty() {
            let all_entries = self.genome.query(&perceived.raw);
            // Also get top entries by confidence regardless of query match
            let top_knowledge: Vec<String> = self
                .genome
                .query("engineering design pattern approach system")
                .iter()
                .take(10)
                .map(|e| {
                    let text = if e.approach.steps.is_empty() {
                        e.approach.approach_type.clone()
                    } else {
                        e.approach.steps.join(" → ")
                    };
                    format!("- {} ({})", text, e.confidence_statement())
                })
                .collect();
            if !top_knowledge.is_empty() {
                perceived.enrichments.insert(
                    "genome.identity".into(),
                    top_knowledge.join("\n"),
                );
            }
            let _ = all_entries; // query side-effect: warms IDF cache
        }

        // HEFP: Epistemic calibration (always inject)
        let profile = self.calibration.epistemic_profile(domain, &JudgmentType::SuccessProbability);
        use hydra_calibration::EpistemicClass;
        let hefp = match &profile.epistemic_class {
            EpistemicClass::WellCalibrated => format!(
                "'{domain}': WELL-CALIBRATED {:.0}% conf. CI90: {:.0}%-{:.0}%, {} obs. {}",
                profile.calibrated_confidence * 100.0,
                profile.credible_interval.0 * 100.0, profile.credible_interval.1 * 100.0,
                profile.observations, profile.methodology),
            EpistemicClass::Uncertain => format!(
                "'{domain}': LIMITED ({} obs, meta {:.0}%). Express moderate confidence.",
                profile.observations, profile.meta_confidence * 100.0),
            EpistemicClass::Uncalibrated => format!("'{domain}': NO DATA. Hedge appropriately."),
            EpistemicClass::Irreducible => format!("'{domain}': STOCHASTIC. Prediction fundamentally limited."),
        };
        perceived.enrichments.insert("calibration.hefp".into(), hefp);

        // Live web search: trigger from comprehension primitives (multilingual, no hardcoded keywords)
        let search_from_primitives = perceived.comprehended.primitives.iter()
            .any(|p| {
                let l = p.label().to_lowercase();
                l.contains("information") || l.contains("discovery") || l.contains("curiosity")
                    || l.contains("learning") || l.contains("exploration")
            });
        // Also trigger for new creative/research/skill domains (proactive immersion)
        let domain_label = perceived.comprehended.primary_domain.label();
        let new_domain_needs_web = matches!(domain_label,
            "creative" | "research" | "skill") && perceived.comprehended.confidence > 0.2;
        if search_from_primitives || new_domain_needs_web {
            // Check knowledge index first — avoid blind search if we know the source
            let strategy = self.knowledge_index.resolution_strategy(&perceived.raw);
            match &strategy {
                crate::web_knowledge::ResolutionStrategy::Indexed { url, reliability } => {
                    eprintln!("hydra: knowledge index hit: {} (rel={:.0}%)", url, reliability * 100.0);
                    perceived.enrichments.insert("web.source".into(),
                        format!("Indexed: {} (reliability {:.0}%)", url, reliability * 100.0));
                }
                _ => {} // Search or Genome — proceed with web search
            }
            let mut web = hydra_web::SearchOrchestrator::new();
            match web.search_blocking(&perceived.raw) {
                Ok(results) => {
                    perceived.enrichments.insert("web.results".into(), results.clone());
                    // Index the result for next time
                    if let Some(url) = results.lines().find(|l| l.starts_with("http")) {
                        self.knowledge_index.add(crate::web_knowledge::KnowledgeSource {
                            topic: perceived.raw.chars().take(60).collect(),
                            url: url.to_string(),
                            source_type: crate::web_knowledge::SourceType::Community,
                            reliability: 0.7,
                            last_accessed: Some(chrono::Utc::now().to_rfc3339()),
                        });
                    }
                }
                Err(e) => eprintln!("hydra: web search: {e}"),
            }
        }

        // GENOME ENRICHMENT with functor-expanded query.
        // Step 1: Expand the raw query with axiom primitive labels.
        // If the functor mapped "netflix" → Risk, we append "risk" to the query.
        // This gives the IDF scorer semantic terms that bridge indirect phrasings.
        let primitive_labels: Vec<&str> = perceived
            .comprehended
            .primitives
            .iter()
            .map(|p| p.label())
            .collect();
        let expanded_query = if primitive_labels.is_empty() {
            perceived.raw.clone()
        } else {
            format!("{} {}", perceived.raw, primitive_labels.join(" "))
        };
        let matches = self.genome.query(&expanded_query);
        if !matches.is_empty() {
            let approaches: Vec<String> = matches
                .iter()
                .take(3)
                .map(|e| {
                    let steps_text = if e.approach.steps.is_empty() {
                        e.approach.approach_type.clone()
                    } else {
                        e.approach.steps.join(" → ")
                    };
                    // CCA: use pre-computed confidence statement (not LLM-generated)
                    format!(
                        "Proven approach ({}): {}\n  CITE EXACTLY: {}",
                        e.confidence_statement(),
                        steps_text,
                        e.confidence_statement(),
                    )
                })
                .collect();
            perceived.enrichments.insert(
                "genome".into(),
                approaches.join("\n"),
            );
        }

        // Detect knowledge gaps — only when genome has no match
        // (casual inputs like "hey" don't need gap tracking)
        if matches.is_empty() && perceived.raw.split_whitespace().count() > 2 {
            let gap_id = self.omniscience.detect_gap(
                domain,
                GapType::Contextual {
                    system: domain.to_string(),
                },
                0.5,
            );
            if !gap_id.is_empty() {
                perceived
                    .enrichments
                    .insert("omniscience.gap".into(), domain.to_string());
            }
        }

        // Oracle projection
        if let Ok(projection) = self.oracle.project(
            &perceived.raw,
            domain,
            &perceived.comprehended.primitives,
        ) {
            if projection.adverse_count() > 0 {
                perceived.enrichments.insert(
                    "oracle".into(),
                    format!(
                        "{} scenarios, {} adverse. {}",
                        projection.scenario_count(),
                        projection.adverse_count(),
                        projection.summary()
                    ),
                );
            }
        }

        // Red team analysis
        if let Ok(scenario) = self.redteam.analyze(
            &perceived.raw,
            &perceived.comprehended.primitives,
        ) {
            if scenario.has_critical_threats() {
                perceived.enrichments.insert(
                    "redteam".into(),
                    format!(
                        "{} threats, go/no-go={}",
                        scenario.threat_count(),
                        scenario.go_no_go.label()
                    ),
                );
            }
        }

        // Judgment Gate: should Hydra act, ask, or refuse?
        let judgment = hydra_wisdom::judge(&hydra_wisdom::JudgmentInput {
            confidence: conf,
            blast_radius: hydra_wisdom::BlastRadius::Contained, // default for queries
            trust_score: 0.9, // from trust field (simplified for now)
            prior_successes: self.exchange_count,
            action_description: perceived.raw.chars().take(60).collect(),
        });
        match &judgment {
            hydra_wisdom::JudgmentDecision::Refuse { reason, .. } => {
                perceived.enrichments.insert(
                    "judgment".into(),
                    format!("REFUSED: {reason}"),
                );
            }
            hydra_wisdom::JudgmentDecision::Ask { reason, .. } => {
                perceived.enrichments.insert(
                    "judgment".into(),
                    format!("NEEDS APPROVAL: {reason}"),
                );
            }
            hydra_wisdom::JudgmentDecision::Act { .. } => {}
        }

        // Wisdom synthesis
        let input = WisdomInput::new(&perceived.raw, domain).with_base_confidence(conf);
        if let Ok(statement) = self.wisdom.synthesize(&input) {
            if statement.is_uncertain {
                let note = statement
                    .key_uncertainties
                    .first()
                    .cloned()
                    .unwrap_or_default();
                perceived
                    .enrichments
                    .insert("wisdom".into(), format!("uncertain: {note}"));
            }
        }

        // FEATURE 1: Surprise detection — fire alert when something is unexpected
        if let Some(surprise) = self.surprise.observe_numeric(
            &format!("confidence:{domain}"),
            conf,
            domain,
        ) {
            perceived.enrichments.insert(
                "surprise".into(),
                format!(
                    "UNEXPECTED: {} (magnitude {:.1})",
                    surprise.summary(),
                    surprise.magnitude
                ),
            );
            eprintln!("hydra: SURPRISE DETECTED — {}", surprise.summary());
        }

        // FEATURE 2: Ask for help — when omniscience has recurring gaps
        let recurring = self.omniscience.recurring_gaps();
        if !recurring.is_empty() {
            let gap_questions: Vec<String> = recurring
                .iter()
                .take(2)
                .map(|g| format!("• I have encountered '{}' {} times without resolution. What is the right approach?", g.topic, g.recurrence))
                .collect();
            if !gap_questions.is_empty() {
                perceived.enrichments.insert(
                    "hydra.questions".into(),
                    format!(
                        "HYDRA NEEDS YOUR HELP — I have knowledge gaps I cannot close on my own:\n{}",
                        gap_questions.join("\n")
                    ),
                );
            }
        }

        // FEATURE 4: Weight of meaning — topics you've invested heavily in
        // get more attention budget (injected as enrichment for the prompt)
        self.exchange_count += 1;
        if self.exchange_count > 10 {
            perceived.enrichments.insert(
                "session.weight".into(),
                format!(
                    "This is exchange {} in this session. The user has invested significant time. Apply extra care and depth.",
                    self.exchange_count
                ),
            );
        }
    }

    fn post_deliver(&mut self, cycle: &CycleResult) {
        if let Err(e) = self.calibration.record_prediction(
            &cycle.domain,
            JudgmentType::SuccessProbability,
            if cycle.success { 0.8 } else { 0.3 },
        ) {
            eprintln!("hydra: intelligence calibration record: {e}");
        }

        // FEATURE 1 continued: track response duration for surprise detection
        self.surprise.observe_numeric(
            "response_duration_ms",
            cycle.duration_ms as f64,
            &cycle.domain,
        );

        // Record genome use — collect IDs first to avoid borrow conflict
        if cycle.success && cycle.enrichments.contains_key("genome") {
            let ids: Vec<String> = self.genome.query(&cycle.intent_summary)
                .iter().take(3).map(|e| e.id.clone()).collect();
            for id in &ids {
                let _ = self.genome.record_use(id, true);
            }
        }
    }
}

/// Simple language detection from Unicode script ranges + common words.
fn detect_language(text: &str) -> String {
    let chars: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    if chars.is_empty() { return "english".into(); }
    for c in &chars {
        if ('\u{4E00}'..='\u{9FFF}').contains(c) { return "chinese".into(); }
        if ('\u{3040}'..='\u{30FF}').contains(c) { return "japanese".into(); }
        if ('\u{AC00}'..='\u{D7AF}').contains(c) { return "korean".into(); }
        if ('\u{0600}'..='\u{06FF}').contains(c) { return "arabic".into(); }
        if ('\u{0400}'..='\u{04FF}').contains(c) { return "russian".into(); }
        if ('\u{0900}'..='\u{097F}').contains(c) { return "hindi".into(); }
    }
    let l = text.to_lowercase();
    if l.contains(" est ") || l.contains(" les ") { return "french".into(); }
    if l.contains(" ist ") || l.contains(" das ") { return "german".into(); }
    if l.contains(" es ") || l.contains(" los ") { return "spanish".into(); }
    if l.contains(" não ") || l.contains(" os ") { return "portuguese".into(); }
    "english".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intelligence_middleware_name() {
        let mw = IntelligenceMiddleware::new();
        assert_eq!(mw.name(), "intelligence");
    }
}

impl Default for IntelligenceMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
