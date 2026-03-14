//! Skill Loader — scans skills/factory/ and skills/custom/ directories
//! and parses TOML skill files into ProfileSkill structs.

use std::path::Path;
use hydra_native_state::operational_profile::ProfileSkill;

/// Load skills from directory-based structure (skills/factory/ + skills/custom/).
/// Falls back to flat skills.toml if directories don't exist.
pub fn load_skills_from_dirs(profile_dir: &Path) -> Vec<ProfileSkill> {
    let skills_dir = profile_dir.join("skills");
    if !skills_dir.is_dir() {
        return Vec::new();
    }

    let mut all = Vec::new();

    // Load factory skills
    let factory = skills_dir.join("factory");
    if factory.is_dir() {
        all.extend(scan_skill_dir(&factory));
    }

    // Load custom skills
    let custom = skills_dir.join("custom");
    if custom.is_dir() {
        all.extend(scan_skill_dir(&custom));
    }

    all
}

/// Recursively scan a directory for .toml files and parse skills from each.
fn scan_skill_dir(dir: &Path) -> Vec<ProfileSkill> {
    let mut skills = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[hydra:skill_loader] Cannot read {}: {}", dir.display(), e);
            return skills;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            skills.extend(scan_skill_dir(&path));
        } else if path.extension().map(|e| e == "toml").unwrap_or(false) {
            match parse_skill_file(&path) {
                Ok(Some(skill)) => skills.push(skill),
                Ok(None) => {} // Valid TOML but not a skill
                Err(e) => eprintln!(
                    "[hydra:skill_loader] Failed to parse {}: {}",
                    path.display(), e
                ),
            }
        }
    }

    skills
}

/// Parse a single TOML skill file into a ProfileSkill.
/// Expects [metadata], [trigger], [steps], [sisters] sections.
pub fn parse_skill_file(path: &Path) -> Result<Option<ProfileSkill>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read error: {}", e))?;
    let table: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("TOML parse error: {}", e))?;

    // Support two formats:
    // 1. [metadata] + [trigger] + [steps].sequence (simple schema)
    // 2. [skill] + [trigger] + [[steps]] array (rich schema)
    let (name, description) = if let Some(metadata) = table.get("metadata") {
        let n = match metadata.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => return Err("metadata.name is required".into()),
        };
        let d = metadata.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        (n, d)
    } else if let Some(skill) = table.get("skill") {
        let n = match skill.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => return Err("skill.name is required".into()),
        };
        let d = skill.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        (n, d)
    } else {
        return Ok(None);
    };

    // Parse trigger patterns
    let trigger_patterns = table.get("trigger")
        .and_then(|t| t.get("patterns"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Build handler from steps — support both formats
    let handler = if let Some(seq) = table.get("steps")
        .and_then(|s| s.get("sequence"))
        .and_then(|v| v.as_array())
    {
        // Simple format: [steps].sequence = ["Step 1", "Step 2"]
        seq.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(" → ")
    } else if let Some(steps_arr) = table.get("steps").and_then(|v| v.as_array()) {
        // Rich format: [[steps]] with name + description fields
        steps_arr.iter()
            .filter_map(|s| {
                let step_name = s.get("name").and_then(|v| v.as_str()).unwrap_or("step");
                let desc = s.get("description").and_then(|v| v.as_str()).unwrap_or("");
                Some(format!("{}: {}", step_name, truncate_step(desc, 80)))
            })
            .collect::<Vec<_>>()
            .join(" → ")
    } else {
        "generic".to_string()
    };

    Ok(Some(ProfileSkill {
        name,
        description,
        trigger_patterns,
        handler,
    }))
}

/// Truncate a step description for handler display.
fn truncate_step(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_skill_file_missing() {
        let result = parse_skill_file(&PathBuf::from("/nonexistent/skill.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_skills_empty_dir() {
        let tmp = std::env::temp_dir().join("hydra_test_skills_empty");
        let _ = std::fs::create_dir_all(&tmp);
        let skills = load_skills_from_dirs(&tmp);
        assert!(skills.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_parse_skill_content() {
        let tmp = std::env::temp_dir().join("hydra_test_skill_parse");
        let _ = std::fs::create_dir_all(&tmp);
        let file = tmp.join("test_skill.toml");
        std::fs::write(&file, r#"
[metadata]
name = "test_skill"
description = "A test skill"

[trigger]
patterns = ["test me"]

[steps]
sequence = ["Step 1", "Step 2"]
"#).unwrap();

        let result = parse_skill_file(&file).unwrap().unwrap();
        assert_eq!(result.name, "test_skill");
        assert_eq!(result.trigger_patterns, vec!["test me"]);
        assert!(result.handler.contains("Step 1"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_parse_rich_skill_format() {
        let tmp = std::env::temp_dir().join("hydra_test_skill_rich");
        let _ = std::fs::create_dir_all(&tmp);
        let file = tmp.join("rich_skill.toml");
        std::fs::write(&file, r#"
[skill]
name = "portfolio_risk"
description = "Assess portfolio risk"

[trigger]
patterns = ["assess risk", "portfolio risk"]

[[steps]]
name = "query_positions"
action = "sister:data"
description = "Retrieve current portfolio positions"

[[steps]]
name = "calculate_var"
action = "compute"
description = "Calculate VaR at 95% and 99% confidence"
"#).unwrap();

        let result = parse_skill_file(&file).unwrap().unwrap();
        assert_eq!(result.name, "portfolio_risk");
        assert_eq!(result.description, "Assess portfolio risk");
        assert_eq!(result.trigger_patterns.len(), 2);
        assert!(result.handler.contains("query_positions"));
        assert!(result.handler.contains("calculate_var"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
