//! Orchestration Designer — creates a skill blueprint from a capability gap.
//! Queries genome for successful patterns in related domains.

use super::CapabilityGap;

/// Blueprint for a self-generated skill.
#[derive(Debug, Clone)]
pub struct SkillBlueprint {
    pub name: String,
    pub domain: String,
    pub approaches: Vec<(String, String)>,  // (situation, approach)
    pub confidence: f64,
}

/// Design a skill blueprint from a capability gap.
/// Queries genome for patterns in related domains and adapts them.
pub fn design_skill(gap: &CapabilityGap, genome: &hydra_genome::GenomeStore) -> SkillBlueprint {
    let mut approaches = Vec::new();

    // Query genome for entries related to the gap domain
    let matches = genome.query(&gap.domain);
    for entry in matches.iter().take(5) {
        let situation = entry.situation.keywords.iter().cloned().collect::<Vec<_>>().join(" ");
        let approach = entry.approach.steps.first().cloned().unwrap_or_default();
        if !situation.is_empty() && !approach.is_empty() {
            approaches.push((
                format!("{} in {}", situation, gap.domain),
                format!("Apply: {} (adapted from related domain)", approach),
            ));
        }
    }

    // If no related patterns found, create a generic approach
    if approaches.is_empty() {
        approaches.push((
            format!("{} task", gap.domain),
            format!("Investigate and learn about {} through web search and practice", gap.domain),
        ));
    }

    let name = format!("auto_{}", gap.domain.replace(' ', "_").replace('/', "_"));
    eprintln!("hydra-evolution: designed skill '{}' with {} approaches", name, approaches.len());

    SkillBlueprint {
        name,
        domain: gap.domain.clone(),
        approaches,
        confidence: 0.5,  // Self-generated starts low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn designs_for_gap() {
        let genome = hydra_genome::GenomeStore::new();
        let gap = CapabilityGap {
            domain: "image_editing".into(), failure_count: 10,
            existing_entries: 2, suggested_approach: String::new(),
        };
        let bp = design_skill(&gap, &genome);
        assert!(bp.name.contains("image_editing"));
        assert!(!bp.approaches.is_empty());
    }

    #[test]
    fn blueprint_has_low_confidence() {
        let genome = hydra_genome::GenomeStore::new();
        let gap = CapabilityGap {
            domain: "test".into(), failure_count: 5,
            existing_entries: 1, suggested_approach: String::new(),
        };
        let bp = design_skill(&gap, &genome);
        assert!((bp.confidence - 0.5).abs() < 0.01);
    }
}
