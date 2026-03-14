//! Sister Synapse — dispatch query to relevant sisters in PARALLEL,
//! collect responses within timeout, inject all into LLM context
//! for cross-sister synthesis.
//!
//! Why isn't a sister doing this? This IS the multi-sister coordination
//! layer. Sisters can't coordinate themselves — this module orchestrates.

use crate::sisters::SistersHandle;
use std::time::Duration;

/// Result of a synapse query across multiple sisters.
#[derive(Debug, Clone)]
pub struct SynapseResult {
    pub query: String,
    pub sister_responses: Vec<SisterResponse>,
    pub total_ms: u64,
    pub sisters_queried: usize,
    pub sisters_responded: usize,
}

/// Response from a single sister.
#[derive(Debug, Clone)]
pub struct SisterResponse {
    pub sister: String,
    pub content: String,
    pub response_ms: u64,
    pub timed_out: bool,
}

/// Configuration for synapse queries.
pub struct SynapseConfig {
    pub timeout: Duration,
    pub max_sisters: usize,
}

impl Default for SynapseConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
            max_sisters: 7,
        }
    }
}

/// Run a synapse query — dispatch to relevant sisters in parallel.
pub async fn synapse_query(
    query: &str,
    sisters: &SistersHandle,
    config: &SynapseConfig,
) -> SynapseResult {
    let start = std::time::Instant::now();

    // Determine which sisters are relevant for this query
    let relevant = select_relevant_sisters(query);
    let sisters_queried = relevant.len().min(config.max_sisters);

    // Dispatch all queries in parallel with timeout
    let mut handles = Vec::new();
    for sister_name in relevant.iter().take(config.max_sisters) {
        let name = sister_name.to_string();
        let q = query.to_string();
        let sh = sisters.clone();
        let timeout = config.timeout;

        handles.push(tokio::spawn(async move {
            let sister_start = std::time::Instant::now();
            let result = tokio::time::timeout(timeout, query_sister(&sh, &name, &q)).await;
            let ms = sister_start.elapsed().as_millis() as u64;

            match result {
                Ok(Some(content)) => SisterResponse {
                    sister: name, content, response_ms: ms, timed_out: false,
                },
                Ok(None) => SisterResponse {
                    sister: name, content: String::new(), response_ms: ms, timed_out: false,
                },
                Err(_) => SisterResponse {
                    sister: name, content: String::new(), response_ms: ms, timed_out: true,
                },
            }
        }));
    }

    // Collect all responses
    let mut responses = Vec::new();
    for handle in handles {
        if let Ok(resp) = handle.await {
            responses.push(resp);
        }
    }

    let responded = responses.iter().filter(|r| !r.content.is_empty() && !r.timed_out).count();
    let total_ms = start.elapsed().as_millis() as u64;

    eprintln!("[hydra:synapse] Query dispatched to {} sisters, {} responded in {}ms",
        sisters_queried, responded, total_ms);

    SynapseResult {
        query: query.to_string(),
        sister_responses: responses,
        total_ms,
        sisters_queried,
        sisters_responded: responded,
    }
}

/// Format synapse result for injection into LLM context.
pub fn format_for_prompt(result: &SynapseResult) -> Option<String> {
    let active: Vec<&SisterResponse> = result.sister_responses.iter()
        .filter(|r| !r.content.is_empty() && !r.timed_out)
        .collect();

    if active.is_empty() {
        return None;
    }

    let mut section = format!(
        "# Cross-Sister Synthesis ({} sisters, {}ms)\n",
        active.len(), result.total_ms,
    );
    for r in &active {
        let content = if r.content.len() > 200 { &r.content[..200] } else { &r.content };
        section.push_str(&format!("  {}: {}\n", r.sister, content));
    }
    section.push_str("Synthesize insights from ALL sister inputs above.\n");
    Some(section)
}

/// Query a specific sister for context about the query.
async fn query_sister(sisters: &SistersHandle, name: &str, query: &str) -> Option<String> {
    match name {
        "Memory" => {
            let results = sisters.memory_query_observations(10).await?;
            Some(results.join("; "))
        }
        "Codebase" => {
            // Use semantic search if available
            sisters.memory_workspace_add(&format!("[synapse-query] codebase: {}", query), "synapse").await;
            None
        }
        "Reality" => {
            sisters.memory_workspace_add(&format!("[synapse-query] reality: {}", query), "synapse").await;
            None
        }
        "Time" => {
            sisters.memory_workspace_add(&format!("[synapse-query] time: {}", query), "synapse").await;
            None
        }
        "Evolve" => {
            sisters.memory_workspace_add(&format!("[synapse-query] evolve: {}", query), "synapse").await;
            None
        }
        _ => None,
    }
}

/// Select which sisters are relevant for a given query.
fn select_relevant_sisters(query: &str) -> Vec<&'static str> {
    let q = query.to_lowercase();
    let mut sisters = Vec::new();

    // Memory is almost always relevant
    sisters.push("Memory");

    if q.contains("code") || q.contains("function") || q.contains("bug") || q.contains("error") {
        sisters.push("Codebase");
    }
    if q.contains("deploy") || q.contains("server") || q.contains("infrastructure") {
        sisters.push("Reality");
        sisters.push("Connect");
    }
    if q.contains("when") || q.contains("timeline") || q.contains("schedule") {
        sisters.push("Time");
    }
    if q.contains("pattern") || q.contains("improve") || q.contains("optimize") {
        sisters.push("Evolve");
    }
    if q.contains("security") || q.contains("risk") || q.contains("vulnerability") {
        sisters.push("Aegis");
    }
    if q.contains("plan") || q.contains("goal") || q.contains("milestone") {
        sisters.push("Planning");
    }

    // Always include at least Memory + one other
    if sisters.len() < 2 {
        sisters.push("Reality");
    }

    sisters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_relevant_code_query() {
        let sisters = select_relevant_sisters("why is this code throwing an error?");
        assert!(sisters.contains(&"Codebase"));
        assert!(sisters.contains(&"Memory"));
    }

    #[test]
    fn test_select_relevant_deploy_query() {
        let sisters = select_relevant_sisters("is the deployment ready?");
        assert!(sisters.contains(&"Reality"));
    }

    #[test]
    fn test_select_minimum() {
        let sisters = select_relevant_sisters("hello");
        assert!(sisters.len() >= 2);
    }

    #[test]
    fn test_format_empty() {
        let result = SynapseResult {
            query: "test".into(),
            sister_responses: vec![],
            total_ms: 100,
            sisters_queried: 3,
            sisters_responded: 0,
        };
        assert!(format_for_prompt(&result).is_none());
    }
}
