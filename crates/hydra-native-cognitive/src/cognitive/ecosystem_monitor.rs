//! Ecosystem Monitor — detects echo chambers, blind spots, and belief
//! diversity issues. Monitors the health of the belief ecosystem.
//!
//! Why isn't a sister doing this? Pure analysis of in-memory belief state.
//! No I/O needed — just statistical analysis of belief distributions.

use hydra_native_state::operational_profile::ProfileBelief;
use std::collections::HashMap;

/// Health assessment of the belief ecosystem.
#[derive(Debug, Clone)]
pub struct EcosystemHealth {
    pub total_beliefs: usize,
    pub domain_count: usize,
    pub confidence_distribution: ConfidenceDistribution,
    pub echo_chamber_risk: f64,
    pub blind_spots: Vec<String>,
    pub warnings: Vec<String>,
    pub diversity_score: f64,
}

/// Distribution of beliefs across confidence levels.
#[derive(Debug, Clone, Default)]
pub struct ConfidenceDistribution {
    pub high: usize,    // >= 0.85
    pub medium: usize,  // 0.5 - 0.85
    pub low: usize,     // < 0.5
}

/// Assess the health of a belief ecosystem.
pub fn assess_health(beliefs: &[ProfileBelief]) -> EcosystemHealth {
    let mut health = EcosystemHealth {
        total_beliefs: beliefs.len(),
        domain_count: 0,
        confidence_distribution: ConfidenceDistribution::default(),
        echo_chamber_risk: 0.0,
        blind_spots: Vec::new(),
        warnings: Vec::new(),
        diversity_score: 0.0,
    };

    if beliefs.is_empty() {
        health.warnings.push("No beliefs loaded — ecosystem is empty".into());
        return health;
    }

    // Confidence distribution
    for b in beliefs {
        if b.confidence >= 0.85 {
            health.confidence_distribution.high += 1;
        } else if b.confidence >= 0.5 {
            health.confidence_distribution.medium += 1;
        } else {
            health.confidence_distribution.low += 1;
        }
    }

    // Domain diversity
    let domains = count_domains(beliefs);
    health.domain_count = domains.len();

    // Echo chamber detection — if >80% of beliefs are in one domain
    let max_domain_pct = domains.values().copied().max().unwrap_or(0) as f64
        / beliefs.len() as f64;
    health.echo_chamber_risk = if max_domain_pct > 0.8 {
        max_domain_pct
    } else if max_domain_pct > 0.6 {
        max_domain_pct * 0.7
    } else {
        0.0
    };

    // Confidence echo chamber — if >85% are high confidence, no self-doubt
    let high_pct = health.confidence_distribution.high as f64 / beliefs.len() as f64;
    if high_pct > 0.85 {
        health.warnings.push(format!(
            "{:.0}% of beliefs are high-confidence — consider adversarial testing \
             to find edge cases",
            high_pct * 100.0,
        ));
        health.echo_chamber_risk = health.echo_chamber_risk.max(high_pct * 0.5);
    }

    // Low confidence warning
    if health.confidence_distribution.low > beliefs.len() / 3 {
        health.warnings.push(format!(
            "{} beliefs have low confidence — consider validating or removing them",
            health.confidence_distribution.low,
        ));
    }

    // Blind spot detection — expected domains that are missing
    let expected = expected_domains_for_context(beliefs);
    for domain in expected {
        if !domains.contains_key(&domain) {
            health.blind_spots.push(format!("No beliefs in '{}' domain", domain));
        }
    }

    // Diversity score (0.0 = monoculture, 1.0 = perfectly distributed)
    health.diversity_score = compute_diversity(&domains, beliefs.len());

    // Overall warnings
    if health.echo_chamber_risk > 0.5 {
        health.warnings.push(format!(
            "Echo chamber risk: {:.0}% — beliefs are concentrated in few domains",
            health.echo_chamber_risk * 100.0,
        ));
    }
    if health.diversity_score < 0.3 {
        health.warnings.push("Low belief diversity — knowledge is narrowly focused".into());
    }

    health
}

/// Format ecosystem health for prompt injection.
pub fn format_for_prompt(health: &EcosystemHealth) -> Option<String> {
    if health.warnings.is_empty() && health.blind_spots.is_empty() {
        return None;
    }

    let mut section = "# Belief Ecosystem Health\n".to_string();
    for w in &health.warnings {
        section.push_str(&format!("  Warning: {}\n", w));
    }
    if !health.blind_spots.is_empty() {
        section.push_str("  Blind spots:\n");
        for s in health.blind_spots.iter().take(3) {
            section.push_str(&format!("    - {}\n", s));
        }
    }
    Some(section)
}

/// Count beliefs per domain.
fn count_domains(beliefs: &[ProfileBelief]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for b in beliefs {
        let domain = b.topic.split('/').next().unwrap_or("general").to_string();
        *counts.entry(domain).or_insert(0) += 1;
    }
    counts
}

/// Shannon diversity index normalized to 0-1.
fn compute_diversity(domains: &HashMap<String, usize>, total: usize) -> f64 {
    if domains.len() <= 1 || total == 0 {
        return 0.0;
    }

    let entropy: f64 = domains.values()
        .map(|&count| {
            let p = count as f64 / total as f64;
            if p > 0.0 { -p * p.ln() } else { 0.0 }
        })
        .sum();

    let max_entropy = (domains.len() as f64).ln();
    if max_entropy > 0.0 { entropy / max_entropy } else { 0.0 }
}

/// Suggest expected domains based on what's present.
fn expected_domains_for_context(beliefs: &[ProfileBelief]) -> Vec<String> {
    let domains: std::collections::HashSet<String> = beliefs.iter()
        .map(|b| b.topic.split('/').next().unwrap_or("").to_string())
        .collect();

    let mut expected = Vec::new();

    // If dev-focused, expect security + testing
    if domains.contains("rust") || domains.contains("python") || domains.contains("typescript") {
        if !domains.contains("security") { expected.push("security".into()); }
        if !domains.contains("debugging") { expected.push("debugging".into()); }
        if !domains.contains("architecture") { expected.push("architecture".into()); }
    }

    // If infra-focused, expect monitoring + reliability
    if domains.contains("cloud") || domains.contains("kubernetes") {
        if !domains.contains("monitoring") { expected.push("monitoring".into()); }
        if !domains.contains("reliability") { expected.push("reliability".into()); }
        if !domains.contains("security") { expected.push("security".into()); }
    }

    // If writing-focused, expect audience + editing
    if domains.contains("writing") || domains.contains("content") {
        if !domains.contains("persuasion") { expected.push("persuasion".into()); }
        if !domains.contains("formats") { expected.push("formats".into()); }
    }

    expected
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(topic: &str, conf: f64) -> ProfileBelief {
        ProfileBelief { topic: topic.into(), content: "test".into(), confidence: conf }
    }

    #[test]
    fn test_empty_ecosystem() {
        let health = assess_health(&[]);
        assert_eq!(health.total_beliefs, 0);
        assert!(!health.warnings.is_empty());
    }

    #[test]
    fn test_echo_chamber_detection() {
        let beliefs: Vec<ProfileBelief> = (0..10).map(|i| b(&format!("rust/{}", i), 0.90)).collect();
        let health = assess_health(&beliefs);
        assert!(health.echo_chamber_risk > 0.5);
    }

    #[test]
    fn test_diversity_score() {
        let beliefs = vec![
            b("rust/a", 0.9), b("rust/b", 0.9),
            b("security/a", 0.9), b("security/b", 0.9),
            b("debug/a", 0.9), b("debug/b", 0.9),
        ];
        let health = assess_health(&beliefs);
        assert!(health.diversity_score > 0.8);
    }

    #[test]
    fn test_blind_spots() {
        let beliefs = vec![b("rust/a", 0.9), b("python/a", 0.9)];
        let health = assess_health(&beliefs);
        assert!(!health.blind_spots.is_empty());
    }
}
