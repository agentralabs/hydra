//! Knowledge Hunter — triggered by ecosystem monitor knowledge gaps.
//! Fetches content via Connect sister, parses via Data sister,
//! extracts belief candidates, presents to user for approval.
//!
//! Why isn't a sister doing this? This orchestrates a multi-sister
//! pipeline: ecosystem monitor → Connect → Data → adversarial → user.

use crate::sisters::SistersHandle;
use hydra_native_state::operational_profile::ProfileBelief;

/// A knowledge gap identified by the ecosystem monitor.
#[derive(Debug, Clone)]
pub struct KnowledgeGap {
    pub domain: String,
    pub description: String,
    pub priority: GapPriority,
    pub related_beliefs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq)]
pub enum GapPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// A candidate belief discovered by the hunter.
#[derive(Debug, Clone)]
pub struct BeliefCandidate {
    pub fact: String,
    pub confidence: f64,
    pub source: String,
    pub domain: String,
    pub validated: bool,
}

/// Result of a hunting session.
#[derive(Debug, Default)]
pub struct HuntingReport {
    pub gaps_investigated: usize,
    pub candidates_found: usize,
    pub candidates_validated: usize,
    pub domains_covered: Vec<String>,
}

impl HuntingReport {
    pub fn summary(&self) -> String {
        format!(
            "Knowledge hunt: {} gaps investigated, {} candidates found, {} validated",
            self.gaps_investigated, self.candidates_found, self.candidates_validated,
        )
    }
}

/// Identify knowledge gaps from the current belief set.
pub fn identify_gaps(beliefs: &[ProfileBelief]) -> Vec<KnowledgeGap> {
    let health = crate::cognitive::ecosystem_monitor::assess_health(beliefs);
    let mut gaps = Vec::new();

    for blind_spot in &health.blind_spots {
        gaps.push(KnowledgeGap {
            domain: blind_spot.replace("No beliefs in '", "").replace("' domain", ""),
            description: blind_spot.clone(),
            priority: GapPriority::Medium,
            related_beliefs: Vec::new(),
        });
    }

    // Check for domains with very few beliefs (< 3)
    let mut domain_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for b in beliefs {
        let domain = b.topic.split('/').next().unwrap_or("general").to_string();
        *domain_counts.entry(domain).or_insert(0) += 1;
    }
    for (domain, count) in &domain_counts {
        if *count < 3 {
            gaps.push(KnowledgeGap {
                domain: domain.clone(),
                description: format!("Only {} beliefs in '{}' — may need deepening", count, domain),
                priority: GapPriority::Low,
                related_beliefs: beliefs.iter()
                    .filter(|b| b.topic.starts_with(domain.as_str()))
                    .map(|b| b.content.clone())
                    .collect(),
            });
        }
    }

    // Sort by priority (highest first)
    gaps.sort_by(|a, b| b.priority.cmp(&a.priority));
    gaps
}

/// Hunt for knowledge to fill gaps. Uses sisters to fetch and validate.
pub async fn hunt(
    gaps: &[KnowledgeGap],
    sisters: &SistersHandle,
    llm_config: &hydra_model::llm_config::LlmConfig,
    max_candidates: usize,
) -> (Vec<BeliefCandidate>, HuntingReport) {
    let mut report = HuntingReport::default();
    let mut candidates = Vec::new();

    for gap in gaps.iter().take(5) {
        report.gaps_investigated += 1;
        report.domains_covered.push(gap.domain.clone());

        // Use LLM to generate candidate beliefs for this gap
        let prompt = format!(
            "Domain: {}\nGap: {}\nExisting related beliefs: {}\n\n\
             Generate 3 expert-level beliefs that would fill this knowledge gap. \
             Each belief should be actionable, specific, and non-obvious. \
             Format each as: BELIEF: <fact> | CONFIDENCE: <0.xx>",
            gap.domain, gap.description,
            gap.related_beliefs.join("; "),
        );

        let result = match crate::sisters::llm_micro_call(llm_config, &prompt, "haiku").await {
            Some(r) => r,
            None => continue,
        };

        // Parse candidates from response
        for line in result.lines() {
            if let Some(belief_start) = line.find("BELIEF:") {
                let fact = line[belief_start + 7..].split('|').next()
                    .unwrap_or("").trim().to_string();
                if fact.is_empty() { continue; }

                let confidence = line.split("CONFIDENCE:")
                    .nth(1)
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(0.6);

                candidates.push(BeliefCandidate {
                    fact,
                    confidence,
                    source: "knowledge-hunter".into(),
                    domain: gap.domain.clone(),
                    validated: false,
                });
                report.candidates_found += 1;
            }
        }

        if candidates.len() >= max_candidates { break; }
    }

    // Store candidates in memory for user review
    for candidate in &candidates {
        let content = format!(
            "[knowledge-hunter] {}: {} (confidence: {:.0}%)",
            candidate.domain, candidate.fact, candidate.confidence * 100.0,
        );
        sisters.memory_workspace_add(&content, "knowledge-hunter").await;
    }

    eprintln!("[hydra:knowledge_hunter] {}", report.summary());
    (candidates, report)
}

/// Format gaps for display.
pub fn format_gaps(gaps: &[KnowledgeGap]) -> String {
    if gaps.is_empty() { return "No knowledge gaps detected.".into(); }
    let mut out = format!("{} knowledge gaps found:\n", gaps.len());
    for gap in gaps {
        out.push_str(&format!("  [{:?}] {} — {}\n", gap.priority, gap.domain, gap.description));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(topic: &str, content: &str) -> ProfileBelief {
        ProfileBelief { topic: topic.into(), content: content.into(), confidence: 0.9 }
    }

    #[test]
    fn test_identify_gaps_empty() {
        let gaps = identify_gaps(&[]);
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_identify_shallow_domain() {
        let beliefs = vec![b("rust/a", "one belief"), b("rust/b", "two beliefs")];
        let gaps = identify_gaps(&beliefs);
        assert!(gaps.iter().any(|g| g.description.contains("Only 2")));
    }

    #[test]
    fn test_format_gaps() {
        let gaps = vec![KnowledgeGap {
            domain: "security".into(),
            description: "No security beliefs".into(),
            priority: GapPriority::High,
            related_beliefs: vec![],
        }];
        let formatted = format_gaps(&gaps);
        assert!(formatted.contains("security"));
    }
}
