//! Gap Detector — finds domains where Hydra consistently fails.
//! Uses genome domain_stats to identify low-confidence areas.

use super::CapabilityGap;

/// Detects capability gaps by analyzing genome domain statistics.
pub struct GapDetector {
    pub min_entries_for_gap: usize,  // Domain must have some entries to show a pattern
    pub confidence_threshold: f64,    // Below this = gap
}

impl GapDetector {
    pub fn new() -> Self {
        Self { min_entries_for_gap: 2, confidence_threshold: 0.4 }
    }

    /// Scan genome for domains with low average confidence.
    pub fn detect(&self, genome: &hydra_genome::GenomeStore) -> Vec<CapabilityGap> {
        let stats = genome.domain_stats();
        let mut gaps = Vec::new();
        for stat in &stats {
            if stat.entry_count >= self.min_entries_for_gap && stat.avg_confidence < self.confidence_threshold {
                gaps.push(CapabilityGap {
                    domain: stat.domain.clone(),
                    failure_count: ((1.0 - stat.avg_confidence) * stat.entry_count as f64) as u64,
                    existing_entries: stat.entry_count,
                    suggested_approach: format!("Improve {} domain with more reliable approaches", stat.domain),
                });
            }
        }
        // Sort by failure count descending (highest impact first)
        gaps.sort_by(|a, b| b.failure_count.cmp(&a.failure_count));
        gaps
    }
}

impl Default for GapDetector {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_genome_no_gaps() {
        let genome = hydra_genome::GenomeStore::new();
        let detector = GapDetector::new();
        let gaps = detector.detect(&genome);
        assert!(gaps.is_empty());
    }

    #[test]
    fn detector_creates() {
        let d = GapDetector::new();
        assert!(d.confidence_threshold > 0.0);
        assert!(d.min_entries_for_gap > 0);
    }

    #[test]
    fn gaps_sorted_by_impact() {
        let mut gaps = vec![
            CapabilityGap { domain: "a".into(), failure_count: 3, existing_entries: 5, suggested_approach: String::new() },
            CapabilityGap { domain: "b".into(), failure_count: 10, existing_entries: 15, suggested_approach: String::new() },
        ];
        gaps.sort_by(|a, b| b.failure_count.cmp(&a.failure_count));
        assert_eq!(gaps[0].domain, "b");
    }
}
