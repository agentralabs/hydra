//! Profile Loader — reads TOML/JSON/MD files from a profile directory
//! and assembles an OperationalProfile.

use std::path::{Path, PathBuf};
use hydra_native_state::operational_profile::*;

/// Load a profile by name from `~/.hydra/profiles/<name>/`.
pub fn load_profile(name: &str) -> Result<OperationalProfile, String> {
    let dir = profiles_dir()
        .ok_or("Cannot determine profiles directory")?
        .join(name);
    if !dir.is_dir() {
        return Err(format!("Profile '{}' not found at {}", name, dir.display()));
    }
    load_profile_from_path(&dir, name)
}

/// Load a profile from an arbitrary directory path.
pub fn load_profile_from_path(path: &Path, name: &str) -> Result<OperationalProfile, String> {
    if !path.is_dir() {
        return Err(format!("Profile path is not a directory: {}", path.display()));
    }

    let mut profile = OperationalProfile {
        name: name.to_string(),
        path: path.to_path_buf(),
        ..Default::default()
    };

    // Every file is optional — missing = default behavior
    profile.identity = load_identity(path);
    profile.beliefs = load_beliefs(path);
    profile.skills = load_skills(path);
    profile.permissions = load_permissions(path);
    profile.goals = load_goals(path);
    profile.connections = load_connections(path);
    profile.model = load_model(path);
    profile.sisters = load_sisters(path);
    profile.prompt_overlay = load_prompt_overlay(path);

    eprintln!("[hydra:profile] Loaded '{}' from {}", name, path.display());
    Ok(profile)
}

/// Validate a profile and return warnings for missing/conflicting settings.
pub fn validate_profile(profile: &OperationalProfile) -> Vec<String> {
    let mut warnings = Vec::new();

    if profile.identity.is_none() && profile.prompt_overlay.is_none() {
        warnings.push("No identity or prompt overlay — Hydra will use default persona.".into());
    }
    if profile.permissions.is_none() {
        warnings.push("No permissions file — using current RuntimeSettings.".into());
    }
    if profile.model.is_none() {
        warnings.push("No model override — using current HYDRA_MODEL.".into());
    }
    if let Some(ref perms) = profile.permissions {
        if perms.shell_exec == Some(true) && perms.sandbox_mode == Some(true) {
            warnings.push("Conflict: shell_exec=true but sandbox_mode=true — sandbox will restrict shells.".into());
        }
    }

    warnings
}

// ── Individual file loaders ──

fn load_identity(dir: &Path) -> Option<ProfileIdentity> {
    let content = read_toml_file(dir, "identity.toml")?;
    let table: toml::Value = toml::from_str(&content).ok()?;
    let id = table.get("identity")?;

    Some(ProfileIdentity {
        persona: id.get("persona")?.as_str()?.to_string(),
        tone: id.get("tone").and_then(|v| v.as_str()).map(|s| s.to_string()),
        constraints: id.get("constraints")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default(),
    })
}

fn load_beliefs(dir: &Path) -> Vec<ProfileBelief> {
    // Try directory-based loading first (beliefs/factory/ + beliefs/learned/)
    let dir_beliefs = super::belief_loader::load_beliefs_from_dirs(dir);
    if !dir_beliefs.is_empty() {
        return dir_beliefs;
    }

    // Backward compat: fall back to flat beliefs.toml
    let content = match read_toml_file(dir, "beliefs.toml") {
        Some(c) => c,
        None => return Vec::new(),
    };
    let table: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let beliefs = match table.get("belief").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    beliefs.iter().filter_map(|b| {
        Some(ProfileBelief {
            topic: b.get("topic")?.as_str()?.to_string(),
            content: b.get("content")?.as_str()?.to_string(),
            confidence: b.get("confidence").and_then(|v| v.as_float()).unwrap_or(0.9),
        })
    }).collect()
}

fn load_skills(dir: &Path) -> Vec<ProfileSkill> {
    // Try directory-based loading first (skills/factory/ + skills/custom/)
    let dir_skills = super::skill_loader::load_skills_from_dirs(dir);
    if !dir_skills.is_empty() {
        return dir_skills;
    }

    // Backward compat: fall back to flat skills.toml
    let content = match read_toml_file(dir, "skills.toml") {
        Some(c) => c,
        None => return Vec::new(),
    };
    let table: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let skills = match table.get("skill").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    skills.iter().filter_map(|s| {
        Some(ProfileSkill {
            name: s.get("name")?.as_str()?.to_string(),
            description: s.get("description")?.as_str()?.to_string(),
            trigger_patterns: s.get("trigger_patterns")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            handler: s.get("handler")?.as_str()?.to_string(),
        })
    }).collect()
}

fn load_permissions(dir: &Path) -> Option<ProfilePermissions> {
    let content = read_toml_file(dir, "permissions.toml")?;
    let table: toml::Value = toml::from_str(&content).ok()?;
    let p = table.get("permissions")?;

    Some(ProfilePermissions {
        file_write: p.get("file_write").and_then(|v| v.as_bool()),
        shell_exec: p.get("shell_exec").and_then(|v| v.as_bool()),
        network_access: p.get("network_access").and_then(|v| v.as_bool()),
        risk_threshold: p.get("risk_threshold").and_then(|v| v.as_str()).map(|s| s.to_string()),
        sandbox_mode: p.get("sandbox_mode").and_then(|v| v.as_bool()),
        max_file_edits: p.get("max_file_edits").and_then(|v| v.as_str()).map(|s| s.to_string()),
        require_approval_critical: p.get("require_approval_critical").and_then(|v| v.as_bool()),
        max_response_tokens: p.get("max_response_tokens").and_then(|v| v.as_integer()).map(|n| n as u32),
        max_file_lines: p.get("max_file_lines").and_then(|v| v.as_integer()).map(|n| n as u32),
        max_agents: p.get("max_agents").and_then(|v| v.as_integer()).map(|n| n as u32),
        max_retry_attempts: p.get("max_retry_attempts").and_then(|v| v.as_integer()).map(|n| n as u8),
        max_context_tokens: p.get("max_context_tokens").and_then(|v| v.as_integer()).map(|n| n as u32),
    })
}

fn load_goals(dir: &Path) -> Vec<ProfileGoal> {
    let content = match read_toml_file(dir, "goals.toml") {
        Some(c) => c,
        None => return Vec::new(),
    };
    let table: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let goals = match table.get("goal").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    goals.iter().filter_map(|g| {
        Some(ProfileGoal {
            title: g.get("title")?.as_str()?.to_string(),
            description: g.get("description")?.as_str()?.to_string(),
            priority: g.get("priority").and_then(|v| v.as_str()).unwrap_or("medium").to_string(),
        })
    }).collect()
}

fn load_connections(dir: &Path) -> Vec<ProfileConnection> {
    let content = match read_toml_file(dir, "connections.toml") {
        Some(c) => c,
        None => return Vec::new(),
    };
    let table: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let conns = match table.get("connection").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    conns.iter().filter_map(|c| {
        Some(ProfileConnection {
            name: c.get("name")?.as_str()?.to_string(),
            host: c.get("host")?.as_str()?.to_string(),
            user: c.get("user").and_then(|v| v.as_str()).map(|s| s.to_string()),
            port: c.get("port").and_then(|v| v.as_integer()).map(|n| n as u16),
            key_path: c.get("key_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }).collect()
}

fn load_model(dir: &Path) -> Option<ProfileModel> {
    let content = read_toml_file(dir, "model.toml")?;
    let table: toml::Value = toml::from_str(&content).ok()?;
    let m = table.get("model")?;

    Some(ProfileModel {
        default: m.get("default")?.as_str()?.to_string(),
        fast: m.get("fast").and_then(|v| v.as_str()).map(|s| s.to_string()),
        provider: m.get("provider").and_then(|v| v.as_str()).map(|s| s.to_string()),
    })
}

fn load_sisters(dir: &Path) -> Option<ProfileSisters> {
    let content = read_toml_file(dir, "sisters.toml")?;
    let table: toml::Value = toml::from_str(&content).ok()?;
    let s = table.get("sisters")?;

    Some(ProfileSisters {
        emphasize: s.get("emphasize")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default(),
        disable: s.get("disable")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default(),
    })
}

fn load_prompt_overlay(dir: &Path) -> Option<String> {
    let path = dir.join("prompt_overlay.md");
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() || trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
        return None; // Empty or only comments
    }
    Some(content)
}

/// Read a TOML file from a profile directory, returning None if missing.
fn read_toml_file(dir: &Path, filename: &str) -> Option<String> {
    let path = dir.join(filename);
    match std::fs::read_to_string(&path) {
        Ok(content) => Some(content),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("[hydra:profile] Warning: could not read {}: {}", path.display(), e);
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_missing_profile() {
        let result = load_profile("nonexistent_profile_xyz_123");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_profile() {
        let profile = OperationalProfile::default();
        let warnings = validate_profile(&profile);
        assert!(warnings.len() >= 2); // identity + permissions
    }
}
