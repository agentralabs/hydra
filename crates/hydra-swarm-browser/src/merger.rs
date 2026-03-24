//! Result merger — deduplicates, ranks, and checks consensus across worker results.
//! Stores merged knowledge into the genome for future instant recall.

use crate::constants::*;
use crate::types::*;

/// Merge multiple worker results into unified knowledge.
pub fn merge_results(results: &[WorkerResult], goal: &SwarmGoal) -> MergedKnowledge {
    // Filter out errors
    let good: Vec<&WorkerResult> = results.iter()
        .filter(|r| r.error.is_none() && !r.content.is_empty())
        .collect();

    if good.is_empty() {
        return MergedKnowledge {
            summary: "No results were successfully retrieved.".into(),
            sources: vec![], confidence: 0.0, worker_count: 0,
        };
    }

    // Deduplicate by content similarity
    let deduped = deduplicate(&good);

    // Build summary from unique results
    let mut summary = format!("# Research: {}\n\n", goal.description);
    let mut sources = Vec::new();

    for (i, result) in deduped.iter().enumerate() {
        // Truncate each result's contribution
        let content = if result.content.len() > 3000 {
            format!("{}...", &result.content[..3000])
        } else {
            result.content.clone()
        };

        summary.push_str(&format!("## Source {} (confidence: {:.0}%)\n", i + 1, result.confidence * 100.0));
        summary.push_str(&content);
        summary.push_str("\n\n");
        sources.push(result.source_url.clone());
    }

    let avg_confidence = deduped.iter().map(|r| r.confidence).sum::<f64>() / deduped.len() as f64;

    MergedKnowledge {
        summary,
        sources,
        confidence: avg_confidence,
        worker_count: good.len(),
    }
}

/// Check consensus among worker results using token overlap.
pub fn check_consensus(results: &[WorkerResult]) -> bool {
    let good: Vec<&WorkerResult> = results.iter()
        .filter(|r| r.error.is_none() && !r.content.is_empty())
        .collect();

    if good.len() < MIN_CONSENSUS_WORKERS { return false; }

    // Tokenize each result
    let token_sets: Vec<Vec<String>> = good.iter()
        .map(|r| tokenize(&r.content))
        .collect();

    // Compute average pairwise Jaccard similarity
    let mut total_sim = 0.0;
    let mut pairs = 0;
    for i in 0..token_sets.len() {
        for j in (i + 1)..token_sets.len() {
            total_sim += jaccard(&token_sets[i], &token_sets[j]);
            pairs += 1;
        }
    }

    if pairs == 0 { return false; }
    let avg_sim = total_sim / pairs as f64;
    avg_sim > 0.15 // Low threshold since different sources will have different phrasing
}

/// Deduplicate results by content similarity.
fn deduplicate<'a>(results: &[&'a WorkerResult]) -> Vec<&'a WorkerResult> {
    let mut unique: Vec<&WorkerResult> = Vec::new();
    for result in results {
        let is_dup = unique.iter().any(|existing| {
            let sim = jaccard(&tokenize(&existing.content), &tokenize(&result.content));
            sim > MERGE_DEDUP_SIMILARITY
        });
        if !is_dup { unique.push(result); }
    }
    // Sort by confidence descending
    unique.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    unique
}

fn jaccard(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() { return 0.0; }
    let set_a: std::collections::HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: std::collections::HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;
    if union > 0.0 { intersection / union } else { 0.0 }
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3)
        .take(200) // Cap for performance
        .map(|w| w.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(content: &str, confidence: f64) -> WorkerResult {
        WorkerResult {
            task_id: uuid::Uuid::new_v4(), worker_id: uuid::Uuid::new_v4(),
            content: content.into(), source_url: "https://example.com".into(),
            confidence, duration_ms: 100, error: None,
        }
    }

    #[test]
    fn merge_empty_results() {
        let goal = SwarmGoal::new("test", 3);
        let merged = merge_results(&[], &goal);
        assert_eq!(merged.worker_count, 0);
    }

    #[test]
    fn merge_filters_errors() {
        let goal = SwarmGoal::new("test", 3);
        let results = vec![
            make_result("good content here", 0.8),
            WorkerResult { error: Some("failed".into()), ..make_result("", 0.0) },
        ];
        let merged = merge_results(&results, &goal);
        assert_eq!(merged.worker_count, 1);
    }

    #[test]
    fn consensus_requires_min_workers() {
        let results = vec![make_result("hello world", 0.8)];
        assert!(!check_consensus(&results)); // need at least 2
    }

    #[test]
    fn consensus_on_similar_content() {
        let results = vec![
            make_result("Rust is a systems programming language focused on safety", 0.8),
            make_result("Rust programming language emphasizes safety and performance", 0.7),
            make_result("The Rust language provides memory safety without garbage collection", 0.9),
        ];
        assert!(check_consensus(&results));
    }

    #[test]
    fn dedup_removes_duplicates() {
        let r1 = make_result("exact same content here for testing purposes", 0.9);
        let r2 = make_result("exact same content here for testing purposes", 0.7);
        let r3 = make_result("completely different topic about cooking recipes", 0.8);
        let refs = vec![&r1, &r2, &r3];
        let deduped = deduplicate(&refs);
        assert_eq!(deduped.len(), 2); // r1 and r3, r2 is duplicate
    }
}
