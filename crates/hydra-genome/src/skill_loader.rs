//! Load genome entries from skill TOML files.
//!
//! Scans the `skills/` directory for skill folders containing `genome.toml`,
//! parses each file, and returns genome entries ready for insertion.

use crate::entry::GenomeEntry;
use crate::signature::ApproachSignature;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Raw TOML representation of a genome.toml file.
#[derive(Debug, Deserialize)]
struct GenomeToml {
    entries: Vec<GenomeTomlEntry>,
}

/// A single entry as written in genome.toml.
#[derive(Debug, Deserialize)]
struct GenomeTomlEntry {
    situation: String,
    approach: String,
    confidence: f64,
    #[serde(default)]
    observations: u64,
    #[serde(default)]
    notes: Option<String>,
}

/// Find the `skills/` directory relative to the repo root.
///
/// Walks up from the current directory looking for a `skills/` folder
/// that contains at least one subdirectory with a `genome.toml`.
fn find_skills_dir() -> Option<PathBuf> {
    // Try current dir first, then walk up
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..6 {
        let candidate = dir.join("skills");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Scan the skills directory and load all genome entries.
///
/// Returns a vec of `(skill_name, Vec<GenomeEntry>)` for each skill
/// that has a valid `genome.toml`.
pub fn load_all_skill_genomes() -> Vec<(String, Vec<GenomeEntry>)> {
    let skills_dir = match find_skills_dir() {
        Some(d) => d,
        None => {
            eprintln!("hydra: skills/ directory not found, skipping skill genome load");
            return Vec::new();
        }
    };

    let mut results = Vec::new();

    let read_dir = match std::fs::read_dir(&skills_dir) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("hydra: failed to read skills/ directory: {}", e);
            return Vec::new();
        }
    };

    for dir_entry in read_dir.flatten() {
        let path = dir_entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let genome_path = path.join("genome.toml");
        if !genome_path.exists() {
            continue;
        }
        match load_genome_toml(&genome_path, &skill_name) {
            Ok(entries) => {
                eprintln!(
                    "hydra: skill '{}' — parsed {} genome entries",
                    skill_name,
                    entries.len()
                );
                results.push((skill_name, entries));
            }
            Err(e) => {
                eprintln!(
                    "hydra: skill '{}' genome.toml parse failed: {}",
                    skill_name, e
                );
            }
        }
    }

    results
}

/// Parse a single genome.toml file into GenomeEntry values.
fn load_genome_toml(
    path: &Path,
    skill_name: &str,
) -> Result<Vec<GenomeEntry>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read error: {}", e))?;
    let parsed: GenomeToml = toml::from_str(&content)
        .map_err(|e| format!("TOML parse error: {}", e))?;

    let entries = parsed
        .entries
        .into_iter()
        .map(|raw| toml_entry_to_genome(raw, skill_name))
        .collect();
    Ok(entries)
}

/// Convert a raw TOML entry to a GenomeEntry.
fn toml_entry_to_genome(raw: GenomeTomlEntry, skill_name: &str) -> GenomeEntry {
    let approach_desc = if let Some(ref notes) = raw.notes {
        format!("{} ({})", raw.approach, notes)
    } else {
        raw.approach.clone()
    };

    let approach = ApproachSignature::new(
        format!("skill.{}", skill_name),
        vec![raw.approach],
        vec![format!("skill:{}", skill_name)],
    );

    let mut entry = GenomeEntry::from_operation(
        &raw.situation,
        approach,
        raw.confidence,
    );

    // Pre-populate observation count as simulated uses
    if raw.observations > 0 {
        entry.use_count = raw.observations;
        let success_rate = raw.confidence.clamp(0.0, 1.0);
        entry.success_count = (raw.observations as f64 * success_rate) as u64;
    }

    // Stash notes in the approach description for richer matching
    if raw.notes.is_some() {
        entry.approach = ApproachSignature::new(
            format!("skill.{}", skill_name),
            vec![approach_desc],
            vec![format!("skill:{}", skill_name)],
        );
    }

    entry
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_genome_toml_roundtrip() {
        let toml_content = r#"
[[entries]]
situation    = "debugging an error"
approach     = "reproduce first"
confidence   = 0.9
observations = 100

[[entries]]
situation    = "new component"
approach     = "types first"
confidence   = 0.85
observations = 50
notes        = "interface before impl"
"#;
        let dir = std::env::temp_dir().join("hydra_genome_test_skill_loader");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("genome.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        let entries = load_genome_toml(&path, "test_skill").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].use_count, 100);
        assert!(entries[0].initial_confidence > 0.89);
        assert_eq!(entries[1].use_count, 50);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
