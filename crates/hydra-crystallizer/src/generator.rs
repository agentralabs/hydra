//! ArtifactGenerator — builds artifacts from crystallization sources.
//! NOT templates. Real operational patterns.

use crate::{
    artifact::{ArtifactKind, CrystallizedArtifact},
    constants::*,
    errors::CrystallizerError,
    source::CrystallizationSource,
};

/// Generates crystallized artifacts from operational sources.
pub struct ArtifactGenerator;

impl ArtifactGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate a playbook from successful execution patterns.
    pub fn generate_playbook(
        &self,
        source: &CrystallizationSource,
    ) -> Result<CrystallizedArtifact, CrystallizerError> {
        if source.successes.len() < MIN_RECORDS_FOR_PLAYBOOK {
            return Err(CrystallizerError::InsufficientData {
                min: MIN_RECORDS_FOR_PLAYBOOK,
                count: source.successes.len(),
            });
        }

        let mut steps = Vec::new();
        steps.push(format!(
            "# Playbook: {} Operations",
            title_case(&source.domain)
        ));
        steps.push(format!(
            "Generated from {} successful executions. Success rate: {:.0}%.\n",
            source.successes.len(),
            source.success_rate() * 100.0,
        ));

        // Pre-execution checklist from avoidable causes
        if !source.avoidable_causes.is_empty() {
            steps.push("## Pre-Execution Checklist".into());
            steps.push("Based on attribution analysis — these caused avoidable overhead:".into());
            for cause in &source.avoidable_causes {
                steps.push(format!("- [ ] Address: {}", cause));
            }
            steps.push(String::new());
        }

        // Proven approaches
        if !source.proven_approaches.is_empty() {
            steps.push("## Proven Approaches".into());
            steps.push(format!(
                "From {} executions (confidence >= {:.0}%):",
                source.total_records(),
                ARTIFACT_PATTERN_CONFIDENCE * 100.0,
            ));
            for (approach, confidence) in &source.proven_approaches {
                if *confidence >= ARTIFACT_PATTERN_CONFIDENCE {
                    steps.push(format!(
                        "- [{:.0}% confidence] {}",
                        confidence * 100.0,
                        approach
                    ));
                }
            }
            steps.push(String::new());
        }

        // Failure patterns to watch for
        if !source.failures.is_empty() {
            steps.push("## Known Failure Modes".into());
            steps.push(format!(
                "Observed in {}/{} executions:",
                source.failures.len(),
                source.total_records()
            ));
            for record in source.failures.iter().take(3) {
                let desc = format!(
                    "task={} action={} attempts={}",
                    record.task_id, record.action_id, record.attempt_count
                );
                let truncated = &desc[..desc.len().min(60)];
                steps.push(format!("- Failed: {}", truncated));
            }
        }

        let content = steps.join("\n");
        let confidence = 0.60 + (source.successes.len() as f64 / 50.0).min(0.30);

        Ok(CrystallizedArtifact::new(
            ArtifactKind::Playbook,
            format!("{} Operations Playbook", title_case(&source.domain)),
            source.domain.clone(),
            content,
            source.total_records(),
            confidence,
        ))
    }

    /// Generate a post-mortem from failure patterns.
    pub fn generate_postmortem(
        &self,
        source: &CrystallizationSource,
        incident_description: &str,
    ) -> Result<CrystallizedArtifact, CrystallizerError> {
        if source.failures.len() < MIN_RECORDS_FOR_POSTMORTEM {
            return Err(CrystallizerError::InsufficientData {
                min: MIN_RECORDS_FOR_POSTMORTEM,
                count: source.failures.len(),
            });
        }

        let mut content = Vec::new();
        content.push(format!("# Post-Mortem: {}", incident_description));
        content.push(format!(
            "\nGenerated from {} failure records in {} domain.\n",
            source.failures.len(),
            source.domain,
        ));

        content.push("## What Happened".into());
        content.push(format!(
            "{} of {} executions resulted in failure ({:.0}% failure rate).",
            source.failures.len(),
            source.total_records(),
            (source.failures.len() as f64 / source.total_records().max(1) as f64) * 100.0,
        ));

        if !source.avoidable_causes.is_empty() {
            content.push("\n## Root Causes (from attribution analysis)".into());
            for cause in &source.avoidable_causes {
                content.push(format!("- {}", cause));
            }
        }

        content.push("\n## Recommendations".into());
        content.push("Based on attribution and pattern analysis:".into());
        for (approach, conf) in &source.proven_approaches {
            if *conf >= ARTIFACT_PATTERN_CONFIDENCE {
                content.push(format!("- [{:.0}%] {}", conf * 100.0, approach));
            }
        }

        let confidence = 0.70;
        Ok(CrystallizedArtifact::new(
            ArtifactKind::PostMortem,
            format!("Post-Mortem: {}", incident_description),
            source.domain.clone(),
            content.join("\n"),
            source.total_records(),
            confidence,
        ))
    }

    /// Generate a knowledge base from acquisition patterns.
    pub fn generate_knowledge_base(
        &self,
        source: &CrystallizationSource,
    ) -> Result<CrystallizedArtifact, CrystallizerError> {
        if source.proven_approaches.is_empty() {
            return Err(CrystallizerError::NoPatterns {
                domain: source.domain.clone(),
            });
        }

        let mut content = Vec::new();
        content.push(format!("# Knowledge Base: {}", title_case(&source.domain)));
        content.push(format!(
            "\nDerived from {} operational records.\n",
            source.total_records(),
        ));

        content.push("## Proven Approaches".into());
        for (approach, confidence) in &source.proven_approaches {
            content.push(format!("**[{:.0}%]** {}", confidence * 100.0, approach));
        }

        if !source.avoidable_causes.is_empty() {
            content.push("\n## Known Pitfalls".into());
            for cause in &source.avoidable_causes {
                content.push(format!("- {}", cause));
            }
        }

        let avg_confidence = source.proven_approaches.iter().map(|(_, c)| c).sum::<f64>()
            / source.proven_approaches.len() as f64;

        Ok(CrystallizedArtifact::new(
            ArtifactKind::KnowledgeBase,
            format!("{} Knowledge Base", title_case(&source.domain)),
            source.domain.clone(),
            content.join("\n"),
            source.total_records(),
            avg_confidence,
        ))
    }
}

impl Default for ArtifactGenerator {
    fn default() -> Self {
        Self::new()
    }
}

fn title_case(s: &str) -> String {
    s.split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::CrystallizationSource;
    use hydra_settlement::{CostClass, CostItem, Outcome, SettlementRecord};

    fn make_success(id: &str, domain: &str) -> SettlementRecord {
        let costs = vec![CostItem::new(CostClass::DirectExecution, 2000, 5.0, 1000)];
        SettlementRecord::new(
            id,
            "deploy.staging",
            domain,
            "deploy service",
            Outcome::Success {
                description: "done".into(),
            },
            costs,
            1000,
            1,
        )
    }

    fn make_failure(id: &str, domain: &str) -> SettlementRecord {
        let costs = vec![CostItem::new(CostClass::DirectExecution, 1000, 3.0, 500)];
        SettlementRecord::new(
            id,
            "deploy.staging",
            domain,
            "deploy service",
            Outcome::HardDenied {
                evidence: "auth rejected".into(),
            },
            costs,
            500,
            1,
        )
    }

    fn build_source(domain: &str, successes: usize, failures: usize) -> CrystallizationSource {
        let mut src = CrystallizationSource::new(domain)
            .with_approach("rotate credentials before deployment", 0.88)
            .with_approach("use blue-green deployment", 0.82)
            .with_avoidable("concurrency:deployment-target");
        for i in 0..successes {
            src = src.with_success(make_success(&format!("s{}", i), domain));
        }
        for i in 0..failures {
            src = src.with_failure(make_failure(&format!("f{}", i), domain));
        }
        src
    }

    #[test]
    fn playbook_generated() {
        let src = build_source("engineering", 8, 2);
        let gen = ArtifactGenerator::new();
        let art = gen.generate_playbook(&src).expect("playbook");
        assert_eq!(art.kind, ArtifactKind::Playbook);
        assert!(art.content.contains("Playbook"));
        assert!(art.confidence >= 0.60);
    }

    #[test]
    fn postmortem_generated() {
        let src = build_source("engineering", 2, 4);
        let gen = ArtifactGenerator::new();
        let art = gen
            .generate_postmortem(&src, "Auth service repeated failures")
            .expect("postmortem");
        assert_eq!(art.kind, ArtifactKind::PostMortem);
        assert!(art.content.contains("Post-Mortem"));
    }

    #[test]
    fn insufficient_data_error() {
        let src = build_source("engineering", 2, 0);
        let gen = ArtifactGenerator::new();
        let result = gen.generate_playbook(&src);
        assert!(matches!(
            result,
            Err(CrystallizerError::InsufficientData { .. })
        ));
    }
}
