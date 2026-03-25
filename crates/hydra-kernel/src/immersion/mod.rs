//! O14 Domain Mastery — 4-phase immersion protocol for rapid domain expertise.
//! Survey → Deep Dive → Practice → Synthesis. Knowledge stored as genome entries.
//! Self-testing engine grades understanding. Cross-domain synthesis finds bridges.

pub mod engine;

use chrono::{DateTime, Utc};
use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};
use std::collections::HashMap;

// Re-export engine functions as the public API
pub use engine::{
    advance_phase, mastery_confidence, is_stale, survey_queries, prefer_free_sources,
    create_domain_entry, detect_contradiction, generate_test_prompt, evaluate_test_difficulty,
    record_self_test, cross_domain_bridges, mastery_summary, load_domain_mastery,
    save_domain_mastery, format_immersion, enrich_prompt_with_immersion,
};

// ── Types ──

/// Immersion phase in the 4-phase protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum ImmersionPhase {
    Survey,
    DeepDive,
    Practice,
    Synthesis,
}

impl ImmersionPhase {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Survey => "survey", Self::DeepDive => "deep-dive",
            Self::Practice => "practice", Self::Synthesis => "synthesis",
        }
    }
}

/// Per-domain mastery state tracked across immersion sessions.
#[derive(Debug, Clone)]
pub struct DomainMastery {
    pub domain: String,
    pub phase: ImmersionPhase,
    pub sources: Vec<DomainSource>,
    pub contradictions: Vec<Contradiction>,
    pub self_test_scores: Vec<f64>,
    pub genome_entry_ids: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl DomainMastery {
    pub fn new(domain: &str) -> Self {
        let now = Utc::now();
        Self {
            domain: domain.to_string(), phase: ImmersionPhase::Survey,
            sources: Vec::new(), contradictions: Vec::new(),
            self_test_scores: Vec::new(), genome_entry_ids: Vec::new(),
            started_at: now, last_updated: now,
        }
    }
}

/// A source discovered during immersion.
#[derive(Debug, Clone)]
pub struct DomainSource {
    pub url: String,
    pub title: String,
    pub content_summary: String,
    pub fetched_at: DateTime<Utc>,
    /// EC-14.1: true if source is freely accessible.
    pub is_free: bool,
}

/// EC-14.2: Two sources contradicting each other on the same topic.
#[derive(Debug, Clone)]
pub struct Contradiction {
    pub topic: String,
    pub source_a: String,
    pub claim_a: String,
    pub source_b: String,
    pub claim_b: String,
    pub resolved: bool,
}

/// Configuration for immersion behaviour.
#[derive(Debug, Clone)]
pub struct ImmersionConfig {
    /// EC-14.5: max concurrent web fetches during immersion.
    pub max_concurrent_fetches: usize,
    /// EC-14.4: entries older than this many days are flagged stale.
    pub staleness_threshold_days: u64,
}

impl Default for ImmersionConfig {
    fn default() -> Self {
        Self { max_concurrent_fetches: 5, staleness_threshold_days: 180 }
    }
}

// ── Middleware ──

/// Domain mastery middleware — enriches the cognitive loop with immersion context.
pub struct ImmersionMiddleware {
    active_domains: HashMap<String, DomainMastery>,
    config: ImmersionConfig,
    current_domain: Option<String>,
}

impl ImmersionMiddleware {
    pub fn new() -> Self {
        Self { active_domains: HashMap::new(), config: ImmersionConfig::default(), current_domain: None }
    }
}

impl CycleMiddleware for ImmersionMiddleware {
    fn name(&self) -> &'static str { "immersion" }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        let domain = perceived.comprehended.primary_domain.label().to_string();
        self.current_domain = Some(domain.clone());
        if !self.active_domains.contains_key(&domain) {
            let genome = hydra_genome::GenomeStore::open();
            if let Some(mastery) = load_domain_mastery(&domain, &genome) {
                self.active_domains.insert(domain.clone(), mastery);
            }
        }
        // EC-14.1: Survey phase — auto-fetch sources via web search
        if let Some(mastery) = self.active_domains.get_mut(&domain) {
            if mastery.phase == ImmersionPhase::Survey && mastery.sources.len() < 3 {
                for q in survey_queries(&mastery.domain).iter().take(3) {
                    let mut web = hydra_web::SearchOrchestrator::new();
                    if let Ok(results) = web.search_blocking(q) {
                        mastery.sources.push(DomainSource {
                            url: q.clone(), title: results.chars().take(100).collect(),
                            content_summary: results.chars().take(300).collect(),
                            fetched_at: chrono::Utc::now(), is_free: true,
                        });
                    }
                }
                if mastery.sources.len() >= 3 {
                    eprintln!("hydra-immersion: survey complete for '{}' ({} sources)", domain, mastery.sources.len());
                }
            }
        }
        if let Some(mastery) = self.active_domains.get(&domain) {
            perceived.enrichments.insert("immersion_context".into(), format_immersion(mastery));
            if is_stale(mastery, &self.config) {
                perceived.enrichments.insert("immersion_stale".into(),
                    format!("Domain '{}' knowledge may be stale (last updated {})",
                        domain, mastery.last_updated.format("%Y-%m-%d")));
            }
        }
    }

    fn enrich_prompt(&self, _perceived: &PerceivedInput) -> Vec<String> {
        if let Some(domain) = &self.current_domain {
            if let Some(mastery) = self.active_domains.get(domain) {
                return enrich_prompt_with_immersion(mastery, &self.config);
            }
        }
        Vec::new()
    }

    fn post_deliver(&mut self, cycle: &CycleResult) {
        let domain = cycle.domain.clone();
        if domain.is_empty() { return; }
        if let Some(mastery) = self.active_domains.get_mut(&domain) {
            advance_phase(mastery);
            let mut genome = hydra_genome::GenomeStore::open();
            save_domain_mastery(mastery, &mut genome);
            let confidence = mastery_confidence(mastery);
            let mut calibration = hydra_calibration::CalibrationEngine::new();
            if let Err(e) = calibration.record_prediction(
                &domain,
                hydra_calibration::JudgmentType::Other("domain_mastery".into()),
                confidence,
            ) {
                eprintln!("hydra-immersion: calibration record failed: {e}");
            }
            eprintln!("hydra-immersion: post_deliver for {} (phase={}, conf={confidence:.2})",
                domain, mastery.phase.label());
        }
    }
}

// ── Public API for TUI commands ──

/// Start a new domain immersion. Returns the initial mastery state.
pub fn start_immersion(domain: &str) -> DomainMastery {
    let mastery = DomainMastery::new(domain);
    let mut genome = hydra_genome::GenomeStore::open();
    save_domain_mastery(&mastery, &mut genome);
    eprintln!("hydra-immersion: started immersion for '{domain}'");
    mastery
}

/// Get mastery status for a domain, or None if no immersion exists.
pub fn get_mastery_status(domain: &str) -> Option<DomainMastery> {
    let genome = hydra_genome::GenomeStore::open();
    load_domain_mastery(domain, &genome)
}

/// List all domains with active immersion.
pub fn list_immersion_domains() -> Vec<String> {
    let genome = hydra_genome::GenomeStore::open();
    let entries = genome.query("immersion mastery");
    entries.iter()
        .filter_map(|e| {
            let desc: String = e.situation.keywords.iter().cloned().collect::<Vec<_>>().join(" ");
            if let Some(rest) = desc.strip_prefix("immersion:") {
                rest.split_whitespace().next().map(|d| d.to_string())
            } else if desc.contains("immersion") {
                e.situation.keywords.iter()
                    .find(|k| *k != "immersion" && *k != "masteri")
                    .cloned()
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mastery(domain: &str) -> DomainMastery { DomainMastery::new(domain) }

    #[test]
    fn phase_survey_to_deep_dive() {
        let mut m = make_mastery("finance");
        m.sources = vec![
            DomainSource { url: "a".into(), title: "A".into(), content_summary: "x".into(), fetched_at: Utc::now(), is_free: true },
            DomainSource { url: "b".into(), title: "B".into(), content_summary: "y".into(), fetched_at: Utc::now(), is_free: true },
            DomainSource { url: "c".into(), title: "C".into(), content_summary: "z".into(), fetched_at: Utc::now(), is_free: false },
        ];
        assert!(advance_phase(&mut m));
        assert_eq!(m.phase, ImmersionPhase::DeepDive);
    }

    #[test]
    fn phase_deep_dive_to_practice() {
        let mut m = make_mastery("solar");
        m.phase = ImmersionPhase::DeepDive;
        m.genome_entry_ids = vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()];
        assert!(advance_phase(&mut m));
        assert_eq!(m.phase, ImmersionPhase::Practice);
    }

    #[test]
    fn phase_practice_to_synthesis() {
        let mut m = make_mastery("ml");
        m.phase = ImmersionPhase::Practice;
        m.self_test_scores = vec![0.95, 0.95, 0.95, 0.95, 0.95];
        assert!(advance_phase(&mut m));
        assert_eq!(m.phase, ImmersionPhase::Synthesis);
    }

    #[test]
    fn phase_practice_stays_if_low_scores() {
        let mut m = make_mastery("ml");
        m.phase = ImmersionPhase::Practice;
        m.self_test_scores = vec![0.4, 0.5, 0.3];
        assert!(!advance_phase(&mut m));
        assert_eq!(m.phase, ImmersionPhase::Practice);
    }

    #[test]
    fn mastery_confidence_empty() {
        let m = make_mastery("test");
        assert!((mastery_confidence(&m) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn mastery_confidence_high() {
        let mut m = make_mastery("test");
        m.self_test_scores = vec![1.0, 1.0, 1.0, 1.0, 1.0];
        assert!(mastery_confidence(&m) > 0.7);
    }

    #[test]
    fn staleness_old_entry() {
        let mut m = make_mastery("law");
        m.last_updated = Utc::now() - chrono::Duration::days(200);
        assert!(is_stale(&m, &ImmersionConfig::default()));
    }

    #[test]
    fn staleness_fresh_entry() {
        assert!(!is_stale(&make_mastery("law"), &ImmersionConfig::default()));
    }

    #[test]
    fn self_test_too_easy_detection() {
        assert!(evaluate_test_difficulty(&[1.0, 1.0, 1.0]) >= 9.0);
    }

    #[test]
    fn contradiction_detected() {
        let existing = vec![DomainSource {
            url: "https://a.com".into(), title: "Solar LCOE Analysis".into(),
            content_summary: "LCOE is $30/MWh".into(), fetched_at: Utc::now(), is_free: true,
        }];
        let new_src = DomainSource {
            url: "https://b.com".into(), title: "Solar LCOE Analysis".into(),
            content_summary: "LCOE is $50/MWh".into(), fetched_at: Utc::now(), is_free: true,
        };
        let c = detect_contradiction(&existing, &new_src).unwrap();
        assert!(!c.resolved);
        assert!(c.claim_a.contains("30"));
        assert!(c.claim_b.contains("50"));
    }

    #[test]
    fn enrichment_includes_phase() {
        let m = make_mastery("test-domain");
        let lines = enrich_prompt_with_immersion(&m, &ImmersionConfig::default());
        assert!(lines.iter().any(|l| l.contains("survey")));
        assert!(lines.iter().any(|l| l.contains("test-domain")));
    }

    #[test]
    fn survey_queries_use_domain() {
        let queries = survey_queries("renewable energy");
        assert_eq!(queries.len(), 3);
        assert!(queries.iter().all(|q| q.contains("renewable energy")));
    }

    #[test]
    fn prefer_free_reorders() {
        let mut sources = vec![
            DomainSource { url: "a".into(), title: "A".into(), content_summary: "".into(), fetched_at: Utc::now(), is_free: false },
            DomainSource { url: "b".into(), title: "B".into(), content_summary: "".into(), fetched_at: Utc::now(), is_free: true },
        ];
        prefer_free_sources(&mut sources);
        assert!(sources[0].is_free);
    }

    #[test]
    fn mastery_summary_format() {
        let summary = mastery_summary(&make_mastery("finance"));
        assert!(summary.contains("finance") && summary.contains("survey"));
    }
}
