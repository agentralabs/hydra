//! O14 Domain Mastery — analysis, persistence, and enrichment functions.
//! Split from mod.rs to respect 400-line limit.

use chrono::{Duration, Utc};
use super::{DomainMastery, DomainSource, Contradiction, ImmersionConfig, ImmersionPhase};

// ── Phase Transition Logic ──

/// Advance mastery to the next phase if conditions are met. Returns true if phase changed.
pub fn advance_phase(mastery: &mut DomainMastery) -> bool {
    let old = mastery.phase.clone();
    mastery.phase = match &mastery.phase {
        ImmersionPhase::Survey if mastery.sources.len() >= 3 => ImmersionPhase::DeepDive,
        ImmersionPhase::DeepDive if mastery.genome_entry_ids.len() >= 5 => ImmersionPhase::Practice,
        ImmersionPhase::Practice => {
            let avg = mastery_confidence(mastery);
            if mastery.self_test_scores.len() >= 3 && avg > 0.7 {
                ImmersionPhase::Synthesis
            } else {
                return false;
            }
        }
        _ => return false,
    };
    if mastery.phase != old {
        mastery.last_updated = Utc::now();
        eprintln!("hydra-immersion: {} advanced to {}", mastery.domain, mastery.phase.label());
        true
    } else {
        false
    }
}

/// Compute mastery confidence from self-test scores (Bayesian average).
pub fn mastery_confidence(mastery: &DomainMastery) -> f64 {
    if mastery.self_test_scores.is_empty() { return 0.0; }
    let prior_strength = 4.0;
    let prior_mean = 0.5;
    let sum: f64 = mastery.self_test_scores.iter().sum();
    let n = mastery.self_test_scores.len() as f64;
    (prior_strength * prior_mean + sum) / (prior_strength + n)
}

/// EC-14.4: Check if mastery data is stale.
pub fn is_stale(mastery: &DomainMastery, config: &ImmersionConfig) -> bool {
    let threshold = Duration::days(config.staleness_threshold_days as i64);
    Utc::now() - mastery.last_updated > threshold
}

// ── Survey Functions ──

/// Generate search queries for the survey phase. No hardcoded keywords — uses domain string.
pub fn survey_queries(domain: &str) -> Vec<String> {
    vec![
        format!("{domain} overview fundamentals"),
        format!("{domain} key concepts and terminology"),
        format!("{domain} best practices and common patterns"),
    ]
}

/// EC-14.1: Reorder sources to prefer freely accessible ones.
pub fn prefer_free_sources(sources: &mut Vec<DomainSource>) {
    sources.sort_by(|a, b| b.is_free.cmp(&a.is_free));
}

// ── Deep Dive Functions ──

/// Create a genome entry for a domain topic discovered during immersion.
pub fn create_domain_entry(
    domain: &str, topic: &str, content: &str,
    genome: &mut hydra_genome::GenomeStore,
) -> Result<String, hydra_genome::GenomeError> {
    let description = format!("immersion:{domain} {topic}");
    let summary = if content.len() > 200 { &content[..200] } else { content };
    let approach = hydra_genome::ApproachSignature::new(
        "domain_mastery",
        vec![summary.to_string()],
        vec!["immersion".to_string(), "web_search".to_string()],
    );
    let id = genome.add_from_operation(&description, approach, 0.6)?;
    eprintln!("hydra-immersion: genome entry for '{topic}' in {domain} (id={id})");
    Ok(id)
}

/// EC-14.2: Check if a new source contradicts any existing source on the same topic.
pub fn detect_contradiction(
    existing: &[DomainSource], new_source: &DomainSource,
) -> Option<Contradiction> {
    for src in existing {
        if src.title.is_empty() || new_source.title.is_empty() { continue; }
        let src_words: Vec<&str> = src.title.split_whitespace().collect();
        let new_words: Vec<&str> = new_source.title.split_whitespace().collect();
        let overlap = src_words.iter().filter(|w| new_words.contains(w)).count();
        let max_len = src_words.len().max(new_words.len());
        if max_len == 0 { continue; }
        let similarity = overlap as f64 / max_len as f64;
        if similarity > 0.5 && src.content_summary != new_source.content_summary
            && !src.content_summary.is_empty() && !new_source.content_summary.is_empty()
        {
            return Some(Contradiction {
                topic: new_source.title.clone(),
                source_a: src.url.clone(), claim_a: src.content_summary.clone(),
                source_b: new_source.url.clone(), claim_b: new_source.content_summary.clone(),
                resolved: false,
            });
        }
    }
    None
}

// ── Self-Testing Engine ──

/// Generate a self-test prompt for the LLM. EC-14.3: professional-certification-level.
pub fn generate_test_prompt(domain: &str, mastery: &DomainMastery) -> String {
    let entry_count = mastery.genome_entry_ids.len();
    let difficulty = evaluate_test_difficulty(&mastery.self_test_scores);
    format!(
        "Generate one professional-certification-level question about {domain}. \
         I have studied {entry_count} topics so far. Difficulty target: {difficulty:.1}/10. \
         Provide the question, then the correct answer, then grade my understanding \
         as a score from 0.0 to 1.0."
    )
}

/// EC-14.3: If all recent scores are 1.0, increase difficulty.
pub fn evaluate_test_difficulty(history: &[f64]) -> f64 {
    if history.is_empty() { return 5.0; }
    let recent: Vec<f64> = history.iter().rev().take(3).copied().collect();
    let avg = recent.iter().sum::<f64>() / recent.len() as f64;
    if avg >= 0.99 { 9.0 } else if avg >= 0.85 { 7.0 } else if avg >= 0.7 { 5.0 } else { 3.0 }
}

/// Record a self-test result: update mastery, genome entries, and calibration.
pub fn record_self_test(
    mastery: &mut DomainMastery, score: f64,
    genome: &mut hydra_genome::GenomeStore,
    calibration: &mut hydra_calibration::CalibrationEngine,
) {
    let clamped = score.clamp(0.0, 1.0);
    mastery.self_test_scores.push(clamped);
    mastery.last_updated = Utc::now();
    let ids: Vec<String> = mastery.genome_entry_ids.clone();
    let success = clamped >= 0.7;
    for id in &ids {
        if let Err(e) = genome.record_use(id, success) {
            eprintln!("hydra-immersion: genome record_use failed for {id}: {e}");
        }
    }
    let confidence = mastery_confidence(mastery);
    if let Err(e) = calibration.record_prediction(
        &mastery.domain,
        hydra_calibration::JudgmentType::Other("domain_mastery".into()),
        confidence,
    ) {
        eprintln!("hydra-immersion: calibration record failed: {e}");
    }
    eprintln!("hydra-immersion: self-test score={clamped:.2} for {} (conf={confidence:.2})", mastery.domain);
}

// ── Synthesis Functions ──

/// Find cross-domain bridges: genome entries from OTHER domains sharing keywords.
pub fn cross_domain_bridges(domain: &str, genome: &hydra_genome::GenomeStore) -> Vec<String> {
    let matches = genome.query(&format!("{domain} patterns connections"));
    matches.iter()
        .filter(|e| {
            let desc: String = e.situation.keywords.iter().cloned().collect::<Vec<_>>().join(" ");
            !desc.contains(&format!("immersion:{}", domain.to_lowercase()))
        })
        .take(5)
        .map(|e| {
            let kw: Vec<&String> = e.situation.keywords.iter().take(5).collect();
            format!("Cross-domain: {} (conf={:.0}%)",
                kw.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", "),
                e.effective_confidence() * 100.0)
        })
        .collect()
}

/// Human-readable mastery summary.
pub fn mastery_summary(mastery: &DomainMastery) -> String {
    let conf = mastery_confidence(mastery);
    let contradictions = mastery.contradictions.iter().filter(|c| !c.resolved).count();
    format!(
        "{}: phase={}, sources={}, entries={}, tests={}, confidence={:.0}%, contradictions={}",
        mastery.domain, mastery.phase.label(), mastery.sources.len(),
        mastery.genome_entry_ids.len(), mastery.self_test_scores.len(),
        conf * 100.0, contradictions,
    )
}

// ── Persistence ──

/// Load domain mastery state from genome.
pub fn load_domain_mastery(domain: &str, genome: &hydra_genome::GenomeStore) -> Option<DomainMastery> {
    let query = format!("immersion:{domain} mastery");
    let matches = genome.query(&query);
    let entry = matches.first()?;
    let mut mastery = DomainMastery::new(domain);
    mastery.started_at = entry.created_at;
    mastery.last_updated = entry.last_used_at;
    mastery.phase = match entry.use_count {
        0..=2 => ImmersionPhase::Survey,
        3..=7 => ImmersionPhase::DeepDive,
        8..=14 => ImmersionPhase::Practice,
        _ => ImmersionPhase::Synthesis,
    };
    Some(mastery)
}

/// Save domain mastery state to genome. Creates or updates the mastery entry.
pub fn save_domain_mastery(mastery: &DomainMastery, genome: &mut hydra_genome::GenomeStore) {
    let description = format!("immersion:{} mastery", mastery.domain);
    let summary = mastery_summary(mastery);
    let matches = genome.query(&description);
    if let Some(entry) = matches.first() {
        let id = entry.id.clone();
        let success = mastery.phase == ImmersionPhase::Synthesis
            || mastery_confidence(mastery) > 0.7;
        if let Err(e) = genome.record_use(&id, success) {
            eprintln!("hydra-immersion: save mastery record_use: {e}");
        }
    } else {
        let approach = hydra_genome::ApproachSignature::new(
            "domain_mastery", vec![summary], vec!["immersion".to_string()],
        );
        match genome.add_from_operation(&description, approach, 0.5) {
            Ok(id) => eprintln!("hydra-immersion: mastery entry created for {} (id={id})", mastery.domain),
            Err(e) => eprintln!("hydra-immersion: mastery entry failed: {e}"),
        }
    }
}

// ── Prompt Enrichment ──

/// Single-line summary for perceived.enrichments.
pub fn format_immersion(mastery: &DomainMastery) -> String {
    let conf = mastery_confidence(mastery);
    format!("domain={}, phase={}, confidence={:.0}%, entries={}",
        mastery.domain, mastery.phase.label(), conf * 100.0, mastery.genome_entry_ids.len())
}

/// Multi-line enrichment for LLM prompt context.
pub fn enrich_prompt_with_immersion(mastery: &DomainMastery, config: &ImmersionConfig) -> Vec<String> {
    let mut lines = vec!["[Domain Mastery]".into()];
    let conf = mastery_confidence(mastery);
    lines.push(format!("  Domain: {} (phase: {})", mastery.domain, mastery.phase.label()));
    lines.push(format!("  Confidence: {:.0}% from {} self-tests", conf * 100.0, mastery.self_test_scores.len()));
    lines.push(format!("  Knowledge: {} genome entries, {} sources", mastery.genome_entry_ids.len(), mastery.sources.len()));
    let unresolved = mastery.contradictions.iter().filter(|c| !c.resolved).count();
    if unresolved > 0 {
        lines.push(format!("  Warning: {} unresolved contradictions in sources", unresolved));
    }
    if is_stale(mastery, config) {
        lines.push(format!("  Warning: domain knowledge last updated {} — may be stale",
            mastery.last_updated.format("%Y-%m-%d")));
    }
    lines
}
