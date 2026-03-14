//! Belief Loader — scans beliefs/factory/ and beliefs/learned/ directories
//! and parses TOML belief files into ProfileBelief structs.

use std::path::Path;
use hydra_native_state::operational_profile::ProfileBelief;

/// Load beliefs from directory-based structure (beliefs/factory/ + beliefs/learned/).
/// Falls back to flat beliefs.toml if directories don't exist.
pub fn load_beliefs_from_dirs(profile_dir: &Path) -> Vec<ProfileBelief> {
    let beliefs_dir = profile_dir.join("beliefs");
    if !beliefs_dir.is_dir() {
        return Vec::new();
    }

    let mut all = Vec::new();

    // Load factory beliefs
    let factory = beliefs_dir.join("factory");
    if factory.is_dir() {
        all.extend(scan_belief_dir(&factory));
    }

    // Load learned beliefs
    let learned = beliefs_dir.join("learned");
    if learned.is_dir() {
        all.extend(scan_belief_dir(&learned));
    }

    all
}

/// Recursively scan a directory for .toml files and parse beliefs from each.
fn scan_belief_dir(dir: &Path) -> Vec<ProfileBelief> {
    let mut beliefs = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[hydra:belief_loader] Cannot read {}: {}", dir.display(), e);
            return beliefs;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            beliefs.extend(scan_belief_dir(&path));
        } else if path.extension().map(|e| e == "toml").unwrap_or(false) {
            match parse_belief_file(&path) {
                Ok(parsed) => beliefs.extend(parsed),
                Err(e) => eprintln!(
                    "[hydra:belief_loader] Failed to parse {}: {}",
                    path.display(), e
                ),
            }
        }
    }

    beliefs
}

/// Parse a single TOML belief file into a vector of ProfileBelief.
/// Expects [[beliefs]] arrays with: fact, confidence, source, action_trigger,
/// edge_case, tags.
pub fn parse_belief_file(path: &Path) -> Result<Vec<ProfileBelief>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read error: {}", e))?;
    let table: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("TOML parse error: {}", e))?;

    let beliefs_arr = match table.get("beliefs").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Ok(Vec::new()),
    };

    // Derive domain from parent directory name for topic
    let domain = path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("general");
    let file_stem = path.file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let topic_prefix = format!("{}/{}", domain, file_stem);

    let beliefs = beliefs_arr.iter().filter_map(|b| {
        let fact = b.get("fact")?.as_str()?;
        let confidence = b.get("confidence")
            .and_then(|v| v.as_float())
            .unwrap_or(0.9);

        Some(ProfileBelief {
            topic: topic_prefix.clone(),
            content: fact.to_string(),
            confidence,
        })
    }).collect();

    Ok(beliefs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_belief_file_missing() {
        let result = parse_belief_file(&PathBuf::from("/nonexistent/beliefs.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_beliefs_empty_dir() {
        let tmp = std::env::temp_dir().join("hydra_test_beliefs_empty");
        let _ = std::fs::create_dir_all(&tmp);
        let beliefs = load_beliefs_from_dirs(&tmp);
        assert!(beliefs.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_parse_belief_content() {
        let tmp = std::env::temp_dir().join("hydra_test_belief_parse");
        let domain = tmp.join("rust");
        let _ = std::fs::create_dir_all(&domain);
        let file = domain.join("test.toml");
        std::fs::write(&file, r#"
[[beliefs]]
fact = "Test belief"
confidence = 0.8
source = "factory"
"#).unwrap();

        let result = parse_belief_file(&file).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "Test belief");
        assert!((result[0].confidence - 0.8).abs() < 0.01);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
