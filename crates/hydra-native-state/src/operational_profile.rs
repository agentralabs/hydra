//! Operational Profiles — loadable config bundles that change Hydra's identity,
//! permissions, model, beliefs, goals, and system prompt without recompilation.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A fully-loaded operational profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationalProfile {
    pub name: String,
    pub path: PathBuf,
    pub identity: Option<ProfileIdentity>,
    pub beliefs: Vec<ProfileBelief>,
    pub skills: Vec<ProfileSkill>,
    pub permissions: Option<ProfilePermissions>,
    pub goals: Vec<ProfileGoal>,
    pub connections: Vec<ProfileConnection>,
    pub model: Option<ProfileModel>,
    pub sisters: Option<ProfileSisters>,
    pub prompt_overlay: Option<String>,
}

/// Identity persona — prepended to system prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileIdentity {
    pub persona: String,
    pub tone: Option<String>,
    pub constraints: Vec<String>,
}

impl Default for ProfileIdentity {
    fn default() -> Self {
        Self {
            persona: String::new(),
            tone: None,
            constraints: Vec::new(),
        }
    }
}

/// A belief to inject via Memory sister.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileBelief {
    pub topic: String,
    pub content: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 { 0.9 }

/// A dynamic skill/capability to register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSkill {
    pub name: String,
    pub description: String,
    pub trigger_patterns: Vec<String>,
    pub handler: String,
}

/// Permission overrides — only `Some` values override RuntimeSettings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfilePermissions {
    pub file_write: Option<bool>,
    pub shell_exec: Option<bool>,
    pub network_access: Option<bool>,
    pub risk_threshold: Option<String>,
    pub sandbox_mode: Option<bool>,
    pub max_file_edits: Option<String>,
    pub require_approval_critical: Option<bool>,
    // UCU #10: Configurable constraints — profile-driven limits
    pub max_response_tokens: Option<u32>,
    pub max_file_lines: Option<u32>,
    pub max_agents: Option<u32>,
    pub max_retry_attempts: Option<u8>,
    pub max_context_tokens: Option<u32>,
}

/// A goal to create via Planning sister.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileGoal {
    pub title: String,
    pub description: String,
    #[serde(default = "default_priority")]
    pub priority: String,
}

fn default_priority() -> String { "medium".into() }

/// SSH/remote connection to pre-establish.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConnection {
    pub name: String,
    pub host: String,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub key_path: Option<String>,
}

/// Model selection override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileModel {
    pub default: String,
    pub fast: Option<String>,
    pub provider: Option<String>,
}

/// Sister emphasis — which sisters to prioritize.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileSisters {
    pub emphasize: Vec<String>,
    pub disable: Vec<String>,
}

/// Profile metadata from profile.toml manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub domain: String,
}

/// Return the base profiles directory: `~/.hydra/profiles/`
pub fn profiles_dir() -> Option<PathBuf> {
    crate::profile::hydra_base_dir().map(|b| b.join("profiles"))
}

/// List all available profile names (subdirectories of profiles_dir).
pub fn list_profiles() -> Vec<String> {
    let Some(dir) = profiles_dir() else { return vec![] };
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    entries.filter_map(|e| {
        let e = e.ok()?;
        if e.file_type().ok()?.is_dir() {
            Some(e.file_name().to_string_lossy().to_string())
        } else {
            None
        }
    }).collect()
}

/// Read the active operational profile name from persisted profile.
pub fn active_profile_name() -> Option<String> {
    let profile = crate::profile::load_profile()?;
    profile.active_operational_profile
}

/// Scaffold a starter profile directory with template files.
/// For custom profiles: creates blank structure with beliefs/learned/, skills/custom/.
pub fn scaffold_profile(name: &str) -> Result<PathBuf, String> {
    let dir = profiles_dir()
        .ok_or("Cannot determine profiles directory")?
        .join(name);

    if dir.exists() {
        return Err(format!("Profile '{}' already exists at {}", name, dir.display()));
    }

    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create profile dir: {}", e))?;

    // Create subdirectories
    for sub in &["beliefs/learned", "skills/custom"] {
        let _ = std::fs::create_dir_all(dir.join(sub));
    }

    // profile.toml
    let profile_meta = format!(
        "[metadata]\nname = \"{}\"\nversion = \"1.0.0\"\ndescription = \"\"\ndomain = \"general\"\n",
        name
    );
    let _ = std::fs::write(dir.join("profile.toml"), profile_meta);

    // identity.toml
    let identity = r#"# Identity — who is Hydra in this profile?
[identity]
persona = "You are a helpful development assistant."
tone = "concise"
constraints = []
"#;
    let _ = std::fs::write(dir.join("identity.toml"), identity);

    // permissions.toml
    let permissions = r#"# Permissions — what can Hydra do?
[permissions]
file_write = true
shell_exec = true
network_access = true
risk_threshold = "medium"
"#;
    let _ = std::fs::write(dir.join("permissions.toml"), permissions);

    // model.toml
    let model = r#"# Model selection
[model]
default = "claude-sonnet-4-6"
"#;
    let _ = std::fs::write(dir.join("model.toml"), model);

    // sisters.toml
    let sisters = "[sisters]\nemphasize = []\ndisable = []\n";
    let _ = std::fs::write(dir.join("sisters.toml"), sisters);

    // goals.toml
    let goals = "# Add goals with [[goal]] entries\n";
    let _ = std::fs::write(dir.join("goals.toml"), goals);

    // prompt_overlay.md
    let overlay = "<!-- Additional instructions appended to the system prompt -->\n";
    let _ = std::fs::write(dir.join("prompt_overlay.md"), overlay);

    Ok(dir)
}

/// Seed factory profiles from source `profiles/` directory to `~/.hydra/profiles/`.
/// Copies full directory trees including beliefs/ and skills/ subdirs.
pub fn seed_profiles(factory_base: &Path) -> Result<usize, String> {
    let target = profiles_dir()
        .ok_or("Cannot determine profiles directory")?;

    std::fs::create_dir_all(&target)
        .map_err(|e| format!("Failed to create profiles dir: {}", e))?;

    let entries = std::fs::read_dir(factory_base)
        .map_err(|e| format!("Cannot read factory profiles: {}", e))?;

    let mut count = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip _schema and hidden dirs
        if name.starts_with('_') || name.starts_with('.') {
            continue;
        }
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let dest = target.join(&name);
        if dest.exists() {
            continue; // Don't overwrite existing profiles
        }
        if let Err(e) = copy_dir_recursive(&entry.path(), &dest) {
            eprintln!("[hydra:profile] Failed to seed '{}': {}", name, e);
            continue;
        }
        eprintln!("[hydra:profile] Seeded factory profile '{}'", name);
        count += 1;
    }
    Ok(count)
}

/// Recursively copy a directory tree.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst)
        .map_err(|e| format!("mkdir {}: {}", dst.display(), e))?;
    let entries = std::fs::read_dir(src)
        .map_err(|e| format!("read {}: {}", src.display(), e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest)
                .map_err(|e| format!("copy {} → {}: {}",
                    path.display(), dest.display(), e))?;
        }
    }
    Ok(())
}

/// Build a human-readable summary of a profile.
pub fn profile_summary(profile: &OperationalProfile) -> String {
    let mut lines = Vec::new();
    lines.push(format!("Profile: {}", profile.name));
    lines.push(format!("Path: {}", profile.path.display()));

    if let Some(ref id) = profile.identity {
        let tone = id.tone.as_deref().unwrap_or("default");
        lines.push(format!("Identity: {} (tone: {})", &id.persona[..id.persona.len().min(60)], tone));
    }
    if let Some(ref perms) = profile.permissions {
        let mut perm_parts = Vec::new();
        if let Some(fw) = perms.file_write { perm_parts.push(format!("file_write={}", fw)); }
        if let Some(se) = perms.shell_exec { perm_parts.push(format!("shell_exec={}", se)); }
        if let Some(rt) = &perms.risk_threshold { perm_parts.push(format!("risk={}", rt)); }
        if !perm_parts.is_empty() {
            lines.push(format!("Permissions: {}", perm_parts.join(", ")));
        }
    }
    if let Some(ref m) = profile.model {
        lines.push(format!("Model: {}", m.default));
    }
    if !profile.beliefs.is_empty() {
        lines.push(format!("Beliefs: {} loaded", profile.beliefs.len()));
    }
    if !profile.goals.is_empty() {
        lines.push(format!("Goals: {} loaded", profile.goals.len()));
    }
    if !profile.skills.is_empty() {
        lines.push(format!("Skills: {} registered", profile.skills.len()));
    }
    if !profile.connections.is_empty() {
        lines.push(format!("Connections: {} configured", profile.connections.len()));
    }
    if profile.prompt_overlay.is_some() {
        lines.push("Prompt overlay: active".to_string());
    }

    lines.join("\n")
}

/// Check if a path looks like a valid profile directory.
pub fn is_valid_profile_dir(path: &Path) -> bool {
    path.is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile() {
        let p = OperationalProfile::default();
        assert!(p.name.is_empty());
        assert!(p.identity.is_none());
        assert!(p.beliefs.is_empty());
    }

    #[test]
    fn test_list_profiles_empty() {
        // Should not panic even if dir doesn't exist
        let _ = list_profiles();
    }

    #[test]
    fn test_profile_summary() {
        let p = OperationalProfile {
            name: "test".into(),
            path: PathBuf::from("/tmp/test"),
            identity: Some(ProfileIdentity {
                persona: "A helpful assistant".into(),
                tone: Some("concise".into()),
                constraints: vec![],
            }),
            model: Some(ProfileModel {
                default: "claude-sonnet-4-6".into(),
                fast: None,
                provider: None,
            }),
            ..Default::default()
        };
        let summary = profile_summary(&p);
        assert!(summary.contains("test"));
        assert!(summary.contains("claude-sonnet"));
    }
}
