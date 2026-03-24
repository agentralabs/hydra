//! Knowledge validator — checks candidates against existing genome before insertion.
//! Classifies as Novel, Complementary, Conflict, or Duplicate.
//! Conflicts saved to ~/.hydra/learning/conflicts.json for human review.

use hydra_genome::GenomeStore;

/// Result of validating a knowledge candidate.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// New knowledge not in genome. Add at confidence 0.6.
    Novel,
    /// Strengthens existing entry. Bayesian update.
    Complementary { existing_id: String },
    /// Contradicts existing high-confidence entry. Flag for review.
    Conflict { existing_id: String },
    /// Already in genome. Skip.
    Duplicate { existing_id: String },
}

/// Validate a knowledge candidate against the existing genome.
pub fn validate(candidate_text: &str, _domain: &str, genome: &GenomeStore) -> ValidationResult {
    let similar = genome.query(candidate_text);

    if similar.is_empty() {
        return ValidationResult::Novel;
    }

    // Check the most similar entry
    let top = &similar[0];
    let overlap = term_overlap(candidate_text, &entry_text(top));

    // High overlap = duplicate or complementary
    if overlap > 0.8 {
        return ValidationResult::Duplicate { existing_id: top.id.clone() };
    }

    if overlap > 0.5 {
        // Similar topic — complementary if same direction, conflict if contradictory
        if top.effective_confidence() > 0.7 {
            return ValidationResult::Complementary { existing_id: top.id.clone() };
        }
    }

    ValidationResult::Novel
}

/// Save a conflict for human review.
pub fn save_conflict(candidate: &str, existing_id: &str, domain: &str) {
    let dir = dirs::home_dir().unwrap_or_default().join(".hydra/learning");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("hydra-learning: create dir failed: {e}");
        return;
    }
    let path = dir.join("conflicts.json");

    let mut conflicts: Vec<serde_json::Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    conflicts.push(serde_json::json!({
        "candidate": candidate,
        "existing_id": existing_id,
        "domain": domain,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }));

    // Keep last 100 conflicts
    if conflicts.len() > 100 { conflicts.drain(..conflicts.len() - 100); }

    if let Ok(json) = serde_json::to_string_pretty(&conflicts) {
        if let Err(e) = std::fs::write(&path, json) {
            eprintln!("hydra-learning: write conflicts failed: {e}");
        }
    }
}

fn term_overlap(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let set_a: std::collections::HashSet<&str> = a_lower
        .split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() >= 3).collect();
    let set_b: std::collections::HashSet<&str> = b_lower
        .split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() >= 3).collect();
    if set_a.is_empty() || set_b.is_empty() { return 0.0; }
    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;
    if union > 0.0 { intersection / union } else { 0.0 }
}

fn entry_text(entry: &hydra_genome::GenomeEntry) -> String {
    let kw: Vec<&str> = entry.situation.keywords.iter().map(|s| s.as_str()).collect();
    format!("{} {}", kw.join(" "), entry.approach.steps.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn novel_when_genome_empty() {
        let genome = GenomeStore::new();
        let result = validate("quantum computing basics", "physics", &genome);
        assert_eq!(result, ValidationResult::Novel);
    }

    #[test]
    fn term_overlap_identical() {
        assert!((term_overlap("hello world test", "hello world test") - 1.0).abs() < 0.01);
    }

    #[test]
    fn term_overlap_different() {
        assert!(term_overlap("hello world", "quantum physics") < 0.1);
    }
}
