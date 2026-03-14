//! Knowledge Fusion — cross-domain insight generation.
//! When beliefs from different domains overlap, NEW knowledge is born
//! that neither domain alone could produce.
//!
//! Why isn't a sister doing this? Orchestrates LLM calls for fusion.
//! Uses Memory sister for storing fused insights.

use crate::sisters::SistersHandle;
use hydra_native_state::operational_profile::ProfileBelief;

/// A fused insight generated from cross-domain beliefs.
#[derive(Debug, Clone)]
pub struct FusedInsight {
    pub content: String,
    pub confidence: f64,
    pub source_domain_a: String,
    pub source_domain_b: String,
    pub parent_belief_a: String,
    pub parent_belief_b: String,
    pub action_trigger: String,
}

/// Report from a fusion session.
#[derive(Debug, Default)]
pub struct FusionReport {
    pub domain_pairs_examined: usize,
    pub insights_generated: usize,
    pub domains_fused: Vec<(String, String)>,
}

impl FusionReport {
    pub fn summary(&self) -> String {
        format!(
            "Fusion: {} domain pairs → {} insights",
            self.domain_pairs_examined, self.insights_generated,
        )
    }
}

/// Run knowledge fusion across belief domains. Finds beliefs that reference
/// similar concepts in different domains and generates cross-domain insights.
pub async fn fuse_domains(
    beliefs: &[ProfileBelief],
    sisters: &SistersHandle,
    llm_config: &hydra_model::LlmConfig,
    max_fusions: usize,
) -> (Vec<FusedInsight>, FusionReport) {
    let mut report = FusionReport::default();
    let mut insights = Vec::new();

    // Group beliefs by domain (top-level topic)
    let domains = group_by_domain(beliefs);
    let domain_names: Vec<&String> = domains.keys().collect();

    // Examine each pair of domains for overlap
    for (i, domain_a) in domain_names.iter().enumerate() {
        for domain_b in domain_names.iter().skip(i + 1) {
            if insights.len() >= max_fusions {
                break;
            }
            report.domain_pairs_examined += 1;

            let beliefs_a = &domains[*domain_a];
            let beliefs_b = &domains[*domain_b];

            // Find overlapping concepts
            let overlap = find_concept_overlap(beliefs_a, beliefs_b);
            if overlap.is_empty() {
                continue;
            }

            // Attempt fusion via LLM
            let a_summary = beliefs_a.iter().take(3)
                .map(|b| format!("- {}", b.content))
                .collect::<Vec<_>>().join("\n");
            let b_summary = beliefs_b.iter().take(3)
                .map(|b| format!("- {}", b.content))
                .collect::<Vec<_>>().join("\n");

            let prompt = format!(
                "Domain A ({}):\n{}\n\nDomain B ({}):\n{}\n\n\
                 These domains share concepts: {}.\n\
                 What insight emerges from their intersection that \
                 NEITHER domain alone would produce?\n\n\
                 Output:\n\
                 INSIGHT: <the cross-domain insight>\n\
                 CONFIDENCE: <0.xx>\n\
                 ACTION: <when to apply this insight>",
                domain_a, a_summary, domain_b, b_summary,
                overlap.join(", "),
            );

            let result = match crate::sisters::llm_micro_call(llm_config, &prompt, "sonnet").await {
                Some(r) => r,
                None => continue,
            };

            let content = match extract_line(&result, "INSIGHT:") {
                Some(c) if !c.is_empty() => c,
                _ => continue,
            };

            let confidence = extract_line(&result, "CONFIDENCE:")
                .and_then(|s| s.trim().parse::<f64>().ok())
                .unwrap_or(0.6);
            let action = extract_line(&result, "ACTION:")
                .unwrap_or_default();

            let insight = FusedInsight {
                content: content.clone(),
                confidence,
                source_domain_a: domain_a.to_string(),
                source_domain_b: domain_b.to_string(),
                parent_belief_a: beliefs_a.first().map(|b| b.content.clone()).unwrap_or_default(),
                parent_belief_b: beliefs_b.first().map(|b| b.content.clone()).unwrap_or_default(),
                action_trigger: action,
            };

            // Store in memory
            let mem_content = format!(
                "[fused] {} + {} → {} (confidence: {:.0}%)",
                domain_a, domain_b, &content[..content.len().min(80)],
                confidence * 100.0,
            );
            sisters.memory_workspace_add(&mem_content, "fused-insights").await;

            insights.push(insight);
            report.insights_generated += 1;
            report.domains_fused.push((domain_a.to_string(), domain_b.to_string()));

            eprintln!("[hydra:fusion] {} × {} → insight (conf: {:.0}%)",
                domain_a, domain_b, confidence * 100.0);
        }
    }

    eprintln!("[hydra:fusion] {}", report.summary());
    (insights, report)
}

/// Group beliefs by their top-level domain.
fn group_by_domain(beliefs: &[ProfileBelief]) -> std::collections::HashMap<String, Vec<&ProfileBelief>> {
    let mut groups: std::collections::HashMap<String, Vec<&ProfileBelief>> = std::collections::HashMap::new();
    for b in beliefs {
        let domain = b.topic.split('/').next().unwrap_or("general").to_string();
        groups.entry(domain).or_default().push(b);
    }
    groups
}

/// Find overlapping concepts between two belief sets.
fn find_concept_overlap(a: &[&ProfileBelief], b: &[&ProfileBelief]) -> Vec<String> {
    let a_words: std::collections::HashSet<String> = a.iter()
        .flat_map(|b| b.content.split_whitespace())
        .filter(|w| w.len() >= 5)
        .map(|w| w.to_lowercase())
        .collect();

    let b_words: std::collections::HashSet<String> = b.iter()
        .flat_map(|b| b.content.split_whitespace())
        .filter(|w| w.len() >= 5)
        .map(|w| w.to_lowercase())
        .collect();

    a_words.intersection(&b_words).cloned().collect()
}

fn extract_line(text: &str, prefix: &str) -> Option<String> {
    text.lines()
        .find(|l| l.trim().starts_with(prefix))
        .map(|l| l.trim()[prefix.len()..].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(topic: &str, content: &str) -> ProfileBelief {
        ProfileBelief { topic: topic.into(), content: content.into(), confidence: 0.85 }
    }

    #[test]
    fn test_group_by_domain() {
        let beliefs = vec![
            b("rust/ownership", "Ownership matters"),
            b("rust/testing", "Tests are good"),
            b("security/vulns", "Input validation"),
        ];
        let groups = group_by_domain(&beliefs);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups["rust"].len(), 2);
    }

    #[test]
    fn test_find_overlap() {
        let a_beliefs = vec![b("rust", "Concurrent access to shared state causes data races")];
        let b_beliefs = vec![b("security", "Data races cause security vulnerabilities in concurrent systems")];
        let a_refs: Vec<&ProfileBelief> = a_beliefs.iter().collect();
        let b_refs: Vec<&ProfileBelief> = b_beliefs.iter().collect();
        let overlap = find_concept_overlap(&a_refs, &b_refs);
        assert!(!overlap.is_empty());
    }
}
