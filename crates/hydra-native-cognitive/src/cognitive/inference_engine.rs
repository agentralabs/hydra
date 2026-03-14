//! Inference Engine — generates new beliefs from existing ones via formal
//! inference chains with confidence propagation.
//!
//! Why isn't a sister doing this? Pure reasoning over in-memory beliefs.
//! Uses LLM for the inference step, Memory sister for storing generated beliefs.

use hydra_native_state::operational_profile::ProfileBelief;

/// A generated belief from inference.
#[derive(Debug, Clone)]
pub struct InferredBelief {
    pub content: String,
    pub confidence: f64,
    pub parent_a: String,
    pub parent_b: String,
    pub inference_type: InferenceType,
    pub falsification: String,
}

/// Type of inference that produced the belief.
#[derive(Debug, Clone)]
pub enum InferenceType {
    ModusPonens,     // A implies B, A is true → B
    Generalization,  // Specific + specific → general
    Specialization,  // General + context → specific prediction
    Contradiction,   // A and B conflict → resolution
}

/// Result of an inference session.
#[derive(Debug, Default)]
pub struct InferenceReport {
    pub pairs_examined: usize,
    pub inferences_generated: usize,
    pub contradictions_found: usize,
    pub avg_confidence: f64,
}

impl InferenceReport {
    pub fn summary(&self) -> String {
        format!(
            "Inference: {} pairs → {} generated, {} contradictions, avg confidence {:.0}%",
            self.pairs_examined, self.inferences_generated,
            self.contradictions_found, self.avg_confidence * 100.0,
        )
    }
}

/// Run inference over belief pairs. Finds beliefs that can be combined
/// to produce new knowledge. Confidence propagates honestly.
pub async fn run_inference(
    beliefs: &[ProfileBelief],
    llm_config: &hydra_model::LlmConfig,
    max_pairs: usize,
) -> (Vec<InferredBelief>, InferenceReport) {
    let mut report = InferenceReport::default();
    let mut inferred = Vec::new();

    // Find promising pairs — beliefs from different domains that share keywords
    let pairs = find_connectable_pairs(beliefs, max_pairs);
    report.pairs_examined = pairs.len();

    for (a, b) in &pairs {
        let prompt = format!(
            "Belief A [{}]: \"{}\"\nBelief B [{}]: \"{}\"\n\n\
             Can these two beliefs be combined to infer something new? \
             If yes, output:\n\
             INFERENCE: <the new insight>\n\
             CONFIDENCE: <product of parent confidences × strength, 0.xx>\n\
             TYPE: <modus_ponens|generalization|specialization|contradiction>\n\
             FALSIFICATION: <what would prove this wrong>\n\n\
             If no meaningful inference, output: NONE",
            a.topic, a.content, b.topic, b.content,
        );

        let result = match crate::sisters::llm_micro_call(llm_config, &prompt, "haiku").await {
            Some(r) => r,
            None => continue,
        };

        if result.contains("NONE") {
            continue;
        }

        let content = extract_line(&result, "INFERENCE:").unwrap_or_default();
        if content.is_empty() {
            continue;
        }

        let max_parent = a.confidence.min(b.confidence);
        let raw_conf = extract_line(&result, "CONFIDENCE:")
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(max_parent * 0.8);
        // Confidence can't exceed the minimum parent — uncertainty multiplies
        let confidence = raw_conf.min(max_parent);

        let inference_type = match extract_line(&result, "TYPE:").as_deref() {
            Some(s) if s.contains("modus") => InferenceType::ModusPonens,
            Some(s) if s.contains("general") => InferenceType::Generalization,
            Some(s) if s.contains("special") => InferenceType::Specialization,
            Some(s) if s.contains("contra") => {
                report.contradictions_found += 1;
                InferenceType::Contradiction
            }
            _ => InferenceType::Generalization,
        };

        let falsification = extract_line(&result, "FALSIFICATION:")
            .unwrap_or_else(|| "No falsification condition identified".into());

        inferred.push(InferredBelief {
            content,
            confidence,
            parent_a: a.content.clone(),
            parent_b: b.content.clone(),
            inference_type,
            falsification,
        });
    }

    report.inferences_generated = inferred.len();
    if !inferred.is_empty() {
        report.avg_confidence = inferred.iter().map(|i| i.confidence).sum::<f64>()
            / inferred.len() as f64;
    }

    eprintln!("[hydra:inference] {}", report.summary());
    (inferred, report)
}

/// Find belief pairs from different domains that share keywords.
fn find_connectable_pairs(beliefs: &[ProfileBelief], max: usize) -> Vec<(&ProfileBelief, &ProfileBelief)> {
    let mut pairs = Vec::new();

    for (i, a) in beliefs.iter().enumerate() {
        for b in beliefs.iter().skip(i + 1) {
            // Different domains
            if a.topic == b.topic {
                continue;
            }
            // Share at least one significant keyword
            let a_words: Vec<&str> = a.content.split_whitespace()
                .filter(|w| w.len() >= 5)
                .collect();
            let b_lower = b.content.to_lowercase();
            let shared = a_words.iter()
                .any(|w| b_lower.contains(&w.to_lowercase()));

            if shared {
                pairs.push((a, b));
                if pairs.len() >= max {
                    return pairs;
                }
            }
        }
    }
    pairs
}

fn extract_line(text: &str, prefix: &str) -> Option<String> {
    text.lines()
        .find(|l| l.trim().starts_with(prefix))
        .map(|l| l.trim()[prefix.len()..].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(topic: &str, content: &str, conf: f64) -> ProfileBelief {
        ProfileBelief { topic: topic.into(), content: content.into(), confidence: conf }
    }

    #[test]
    fn test_find_pairs_different_domains() {
        let beliefs = vec![
            b("rust", "Ownership prevents data races in concurrent code", 0.95),
            b("security", "Data races cause security vulnerabilities in concurrent systems", 0.90),
        ];
        let pairs = find_connectable_pairs(&beliefs, 10);
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn test_find_pairs_same_domain_skipped() {
        let beliefs = vec![
            b("rust", "Ownership is important", 0.9),
            b("rust", "Borrowing complements ownership", 0.9),
        ];
        let pairs = find_connectable_pairs(&beliefs, 10);
        assert_eq!(pairs.len(), 0);
    }

    #[test]
    fn test_report() {
        let report = InferenceReport {
            pairs_examined: 10,
            inferences_generated: 3,
            contradictions_found: 1,
            avg_confidence: 0.72,
        };
        assert!(report.summary().contains("10 pairs"));
    }
}
