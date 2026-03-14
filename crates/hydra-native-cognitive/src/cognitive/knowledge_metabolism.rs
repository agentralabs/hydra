//! Knowledge Metabolism — observations → crystallizations → principles.
//! Runs in Dream State (idle time). Beliefs evolve through a lifecycle.
//!
//! Why isn't a sister doing this? Uses Memory sister for storage + LLM for
//! crystallization. This module orchestrates the metabolism pipeline.

use crate::sisters::SistersHandle;

/// A belief at a specific metabolic stage.
#[derive(Debug, Clone)]
pub struct MetabolicBelief {
    pub content: String,
    pub stage: MetabolicStage,
    pub confidence: f64,
    pub source_count: usize,
    pub domain: String,
}

/// Lifecycle stages of a belief.
#[derive(Debug, Clone, PartialEq)]
pub enum MetabolicStage {
    Observation,     // Raw input, low abstraction
    Crystallization, // Pattern from 5+ observations
    Principle,       // Cross-pattern abstraction from 3+ crystallizations
}

/// Result of a metabolism cycle.
#[derive(Debug, Default)]
pub struct MetabolismReport {
    pub observations_processed: usize,
    pub crystallizations_created: usize,
    pub principles_extracted: usize,
    pub domains_touched: Vec<String>,
}

impl MetabolismReport {
    pub fn summary(&self) -> String {
        format!(
            "Metabolism: {} obs → {} crystals → {} principles (domains: {})",
            self.observations_processed,
            self.crystallizations_created,
            self.principles_extracted,
            self.domains_touched.join(", "),
        )
    }
}

/// Run a metabolism cycle: scan observations, crystallize patterns, extract principles.
/// Called during Dream State (idle time).
pub async fn metabolize(
    sisters: &SistersHandle,
    llm_config: &hydra_model::LlmConfig,
    max_observations: usize,
) -> MetabolismReport {
    let mut report = MetabolismReport::default();

    // 1. Query recent observations from Memory sister
    let observations = match sisters.memory_query_observations(max_observations).await {
        Some(obs) => obs,
        None => {
            eprintln!("[hydra:metabolism] No observations from Memory sister");
            return report;
        }
    };
    report.observations_processed = observations.len();

    if observations.len() < 5 {
        eprintln!("[hydra:metabolism] Need 5+ observations for crystallization, have {}", observations.len());
        return report;
    }

    // 2. Cluster observations by domain similarity
    let clusters = cluster_by_domain(&observations);

    // 3. For clusters with 5+ members, attempt crystallization
    for (domain, cluster) in &clusters {
        if cluster.len() < 5 {
            continue;
        }
        report.domains_touched.push(domain.clone());

        let prompt = format!(
            "Given these {} observations about '{}':\n{}\n\n\
             What pattern do they reveal? State it as a single principle \
             with confidence 0.0-1.0. Format: PATTERN: <the pattern> | CONFIDENCE: <0.xx>",
            cluster.len(), domain, cluster.join("\n- "),
        );

        let crystal = match crate::sisters::llm_micro_call(llm_config, &prompt, "haiku").await {
            Some(r) => r,
            None => continue,
        };

        // Store crystallization in Memory sister
        let content = format!("[crystallized] {}", crystal);
        sisters.memory_workspace_add(&content, domain).await;
        report.crystallizations_created += 1;

        eprintln!("[hydra:metabolism] Crystallized in '{}': {}", domain,
            &crystal[..crystal.len().min(80)]);
    }

    // 4. Query existing crystallizations, attempt principle extraction
    if report.crystallizations_created >= 3 {
        let crystals = sisters.memory_query_crystallizations(20).await;
        if let Some(crystal_list) = crystals {
            if crystal_list.len() >= 3 {
                let prompt = format!(
                    "Given these crystallized patterns:\n{}\n\n\
                     Extract the highest-level principle that connects them. \
                     Format: PRINCIPLE: <the principle> | CONFIDENCE: <0.xx>",
                    crystal_list.join("\n- "),
                );

                if let Some(principle) = crate::sisters::llm_micro_call(llm_config, &prompt, "sonnet").await {
                    let content = format!("[principle] {}", principle);
                    sisters.memory_workspace_add(&content, "principles").await;
                    report.principles_extracted += 1;
                    eprintln!("[hydra:metabolism] Principle extracted: {}",
                        &principle[..principle.len().min(80)]);
                }
            }
        }
    }

    eprintln!("[hydra:metabolism] {}", report.summary());
    report
}

/// Cluster observations by extracting domain keywords.
fn cluster_by_domain(observations: &[String]) -> Vec<(String, Vec<String>)> {
    let mut clusters: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for obs in observations {
        let domain = extract_domain(obs);
        clusters.entry(domain).or_default().push(obs.clone());
    }

    clusters.into_iter().collect()
}

/// Extract domain keyword from an observation string.
fn extract_domain(text: &str) -> String {
    // Look for [domain] tags or topic: prefixes
    if let Some(start) = text.find('[') {
        if let Some(end) = text[start..].find(']') {
            return text[start + 1..start + end].to_lowercase();
        }
    }
    if let Some(idx) = text.find(':') {
        let prefix = text[..idx].trim().to_lowercase();
        if prefix.len() < 30 {
            return prefix;
        }
    }
    "general".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain_bracketed() {
        assert_eq!(extract_domain("[rust] ownership matters"), "rust");
    }

    #[test]
    fn test_extract_domain_colon() {
        assert_eq!(extract_domain("deployment: canary is safer"), "deployment");
    }

    #[test]
    fn test_extract_domain_fallback() {
        assert_eq!(extract_domain("just some text"), "general");
    }

    #[test]
    fn test_cluster_by_domain() {
        let obs = vec![
            "[rust] ownership".into(),
            "[rust] borrowing".into(),
            "[deploy] canary".into(),
        ];
        let clusters = cluster_by_domain(&obs);
        assert!(clusters.len() >= 2);
    }

    #[test]
    fn test_metabolism_report() {
        let report = MetabolismReport {
            observations_processed: 10,
            crystallizations_created: 2,
            principles_extracted: 0,
            domains_touched: vec!["rust".into()],
        };
        assert!(report.summary().contains("10 obs"));
    }
}
