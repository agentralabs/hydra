//! LegacyBuilder — constructs legacy artifacts from succession packages
//! and operational history.

use crate::{
    artifact::{LegacyArtifact, LegacyKind},
    constants::{LEGACY_CONFIDENCE_FLOOR, MIN_DAYS_FOR_LEGACY},
    errors::LegacyError,
};
use hydra_succession::SuccessionPackage;

/// Builds legacy artifacts.
pub struct LegacyBuilder;

impl LegacyBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Build a knowledge record from a succession package's genome.
    pub fn knowledge_record(
        &self,
        package: &SuccessionPackage,
        domain: &str,
    ) -> Result<LegacyArtifact, LegacyError> {
        if package.wisdom_days < MIN_DAYS_FOR_LEGACY {
            return Err(LegacyError::InsufficientHistory {
                days: package.wisdom_days,
                min: MIN_DAYS_FOR_LEGACY,
            });
        }

        // Filter genome entries for this domain
        let domain_entries: Vec<_> = package
            .genome
            .entries
            .iter()
            .filter(|e| e.domain.contains(domain))
            .collect();

        let avg_confidence = if domain_entries.is_empty() {
            0.0
        } else {
            domain_entries.iter().map(|e| e.confidence).sum::<f64>()
                / domain_entries.len() as f64
        };

        let mut content_lines = Vec::new();
        content_lines.push(format!(
            "# Knowledge Record: {} Domain",
            title_case(domain),
        ));
        content_lines.push(format!(
            "Source: {} operational days. {} genome entries. Lineage: {}.\n",
            package.wisdom_days,
            domain_entries.len(),
            package.lineage_id,
        ));
        content_lines.push("## Proven Approaches".into());
        for entry in domain_entries.iter().take(20) {
            if entry.confidence >= LEGACY_CONFIDENCE_FLOOR {
                content_lines.push(format!(
                    "- [{:.0}% | {} obs] {} → {}",
                    entry.confidence * 100.0,
                    entry.observations,
                    entry.situation,
                    entry.approach,
                ));
            }
        }
        if domain_entries.len() > 20 {
            content_lines.push(format!(
                "\n...and {} more entries (confidence >= {:.0}%).",
                domain_entries.len() - 20,
                LEGACY_CONFIDENCE_FLOOR * 100.0,
            ));
        }

        Ok(LegacyArtifact::new(
            &package.lineage_id,
            LegacyKind::KnowledgeRecord {
                domain: domain.to_string(),
            },
            format!(
                "{} Knowledge Record — {} Days",
                title_case(domain),
                package.wisdom_days
            ),
            content_lines.join("\n"),
            package.wisdom_days,
            domain_entries.len(),
            avg_confidence,
        ))
    }

    /// Build an operational record summarizing what was done.
    pub fn operational_record(
        &self,
        package: &SuccessionPackage,
        period_description: &str,
    ) -> Result<LegacyArtifact, LegacyError> {
        if package.wisdom_days < MIN_DAYS_FOR_LEGACY {
            return Err(LegacyError::InsufficientHistory {
                days: package.wisdom_days,
                min: MIN_DAYS_FOR_LEGACY,
            });
        }

        let content = format!(
            "# Operational Record: {}\n\
             Lineage: {}\n\
             Period: {} operational days\n\
             Genome entries accumulated: {}\n\
             Soul entries accumulated: {}\n\
             Calibration profiles: {}\n\n\
             ## Summary\n\
             This entity operated for {} days, accumulating {} genome entries \
             across {} domains and {} soul orientation entries. Calibration data \
             covers {} domains with observed bias corrections.\n\n\
             The morphic signature depth is {} days, providing continuous \
             identity proof from first boot.",
            period_description,
            package.lineage_id,
            package.wisdom_days,
            package.genome_entry_count(),
            package.soul_entry_count(),
            package.calibration_profile_count(),
            package.wisdom_days,
            package.genome_entry_count(),
            package.genome.total_domains,
            package.soul_entry_count(),
            package.calibration_profile_count(),
            package.morphic.days_depth,
        );

        Ok(LegacyArtifact::new(
            &package.lineage_id,
            LegacyKind::OperationalRecord {
                period_description: period_description.to_string(),
            },
            format!("Operational Record — {} Days", package.wisdom_days),
            content,
            package.wisdom_days,
            package.genome_entry_count() + package.soul_entry_count(),
            0.85,
        ))
    }

    /// Build a wisdom record from high-confidence calibration data.
    pub fn wisdom_record(
        &self,
        package: &SuccessionPackage,
        domain: &str,
    ) -> Result<LegacyArtifact, LegacyError> {
        if package.wisdom_days < MIN_DAYS_FOR_LEGACY {
            return Err(LegacyError::InsufficientHistory {
                days: package.wisdom_days,
                min: MIN_DAYS_FOR_LEGACY,
            });
        }

        let domain_profiles: Vec<_> = package
            .calibration
            .profiles
            .iter()
            .filter(|p| p.is_significant)
            .collect();

        let mut content_lines = Vec::new();
        content_lines.push(format!(
            "# Wisdom Record: {} Domain",
            title_case(domain)
        ));
        content_lines.push(format!(
            "Source: {} operational days, {} calibration profiles.\n",
            package.wisdom_days,
            domain_profiles.len(),
        ));
        content_lines.push("## Known Judgment Biases (Calibrated)".into());
        for profile in &domain_profiles {
            let direction = if profile.mean_offset < 0.0 {
                format!(
                    "overconfident by {:.0}%",
                    profile.mean_offset.abs() * 100.0
                )
            } else {
                format!("underconfident by {:.0}%", profile.mean_offset * 100.0)
            };
            content_lines.push(format!(
                "- {}/{}: {} (n={})",
                profile.domain, profile.judgment_type, direction, profile.sample_size,
            ));
        }
        content_lines.push(format!(
            "\nTotal records tracked: {}",
            package.calibration.total_records_tracked,
        ));

        Ok(LegacyArtifact::new(
            &package.lineage_id,
            LegacyKind::WisdomRecord {
                domain: domain.to_string(),
            },
            format!(
                "{} Wisdom Record — {} Days",
                title_case(domain),
                package.wisdom_days
            ),
            content_lines.join("\n"),
            package.wisdom_days,
            domain_profiles.len(),
            0.82,
        ))
    }
}

impl Default for LegacyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_succession::{InstanceState, SuccessionExporter};

    fn mature_package() -> SuccessionPackage {
        let exporter = SuccessionExporter::new();
        exporter
            .export(&InstanceState {
                instance_id: "hydra-v1".into(),
                lineage_id: "hydra-agentra-lineage".into(),
                days_running: 7300,
                soul_entries: 500,
                genome_entries: 2_400,
                calibration_profiles: 47,
            })
            .expect("should export mature package")
    }

    #[test]
    fn knowledge_record_built() {
        let pkg = mature_package();
        let builder = LegacyBuilder::new();
        let art = builder
            .knowledge_record(&pkg, "engineering")
            .expect("should build knowledge record");
        assert!(art.verify_integrity());
        assert!(art.content.contains("Knowledge Record"));
    }

    #[test]
    fn operational_record_built() {
        let pkg = mature_package();
        let builder = LegacyBuilder::new();
        let art = builder
            .operational_record(&pkg, "20-year operational history")
            .expect("should build operational record");
        assert!(art.content.contains("Operational Record"));
        assert_eq!(art.source_days, 7300);
    }

    #[test]
    fn insufficient_history_error() {
        let exporter = SuccessionExporter::new();
        let pkg = exporter
            .export(&InstanceState {
                instance_id: "h".into(),
                lineage_id: "l".into(),
                days_running: 30,
                soul_entries: 5,
                genome_entries: 10,
                calibration_profiles: 1,
            })
            .expect("should export small package");
        let builder = LegacyBuilder::new();
        let r = builder.knowledge_record(&pkg, "test");
        assert!(matches!(r, Err(LegacyError::InsufficientHistory { .. })));
    }
}
