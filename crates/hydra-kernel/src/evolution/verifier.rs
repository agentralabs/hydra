//! Skill Verifier — validates self-generated skills before loading.
//! Checks: TOML parses, entries have valid confidence, no genome duplicates.

/// Validate a generated skill file. Returns the number of valid entries.
pub fn validate_skill(path: &str, genome: &hydra_genome::GenomeStore) -> Result<usize, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {path}: {e}"))?;

    let parsed: SkillFile = toml::from_str(&content)
        .map_err(|e| format!("Invalid TOML in {path}: {e}"))?;

    if parsed.entry.is_empty() {
        return Err("Skill has no entries".into());
    }

    let mut valid_count = 0;
    for entry in &parsed.entry {
        // Validate confidence range
        if entry.confidence < 0.0 || entry.confidence > 1.0 {
            return Err(format!("Invalid confidence {}: must be 0.0-1.0", entry.confidence));
        }
        // Validate non-empty fields
        if entry.situation.trim().is_empty() || entry.approach.trim().is_empty() {
            return Err("Entry has empty situation or approach".into());
        }
        // Check for duplicates in genome (BM25 similarity)
        let matches = genome.query(&entry.situation);
        if let Some(top) = matches.first() {
            let overlap = hydra_genome::SituationSignature::from_description(&entry.situation)
                .similarity(&top.situation);
            if overlap > 0.9 {
                eprintln!("hydra-evolution: skipping duplicate entry for '{}'", entry.situation);
                continue; // Skip but don't fail
            }
        }
        valid_count += 1;
    }

    if valid_count == 0 {
        return Err("All entries were duplicates".into());
    }

    eprintln!("hydra-evolution: validated {valid_count}/{} entries in {path}", parsed.entry.len());
    Ok(valid_count)
}

#[derive(serde::Deserialize)]
struct SkillFile {
    #[serde(default)]
    entry: Vec<SkillEntry>,
}

#[derive(serde::Deserialize)]
struct SkillEntry {
    situation: String,
    approach: String,
    #[serde(default = "default_confidence")]
    confidence: f64,
}

fn default_confidence() -> f64 { 0.5 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_toml_structure() {
        let toml = r#"
[[entry]]
situation = "test situation"
approach = "test approach"
confidence = 0.5
"#;
        let parsed: SkillFile = toml::from_str(toml).unwrap();
        assert_eq!(parsed.entry.len(), 1);
        assert_eq!(parsed.entry[0].situation, "test situation");
    }

    #[test]
    fn empty_entries_fail() {
        let toml = "# empty";
        let parsed: SkillFile = toml::from_str(toml).unwrap();
        assert!(parsed.entry.is_empty());
    }
}
