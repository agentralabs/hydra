//! Intelligence middleware — calibration, wisdom, oracle, omniscience, redteam, genome.
//!
//! Enriches the cognitive loop with judgment, projection, gap detection,
//! adversarial analysis, and genome-proven approaches.
//! All per-request, all non-blocking.

use hydra_calibration::{CalibrationEngine, JudgmentType};
use hydra_genome::GenomeStore;
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
    exchange_count: u64,
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
            exchange_count: 0,
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

        // Calibrate confidence
        let adjusted = self.calibration.calibrate(
            conf,
            domain,
            &JudgmentType::SuccessProbability,
        );
        if adjusted.changed_significantly() {
            perceived.enrichments.insert(
                "calibration".into(),
                format!(
                    "raw={:.2} calibrated={:.2} reliable={}",
                    adjusted.raw, adjusted.calibrated, adjusted.is_reliable
                ),
            );
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

        // Detect knowledge gaps
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
                .insert("omniscience.gap".into(), gap_id);
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
