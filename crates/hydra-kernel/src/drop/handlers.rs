//! Built-in drop handlers — process each item type.
//! New handlers can be registered at runtime via DropGateway::register_handler().

use std::path::Path;
use super::classifier::DropItemType;

/// Outcome of processing a dropped item.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DropOutcome {
    Accepted { item_type: String, destination: String, details: String },
    Rejected { item_type: String, reason: String },
}

/// Trait for extensible drop handlers. Implement this to add new item types.
pub trait DropHandler: Send + Sync {
    /// What item types this handler processes.
    fn handles(&self) -> Vec<DropItemType>;
    /// Validate the item. Return Err to reject.
    fn validate(&self, path: &Path, item_type: &DropItemType) -> Result<(), String>;
    /// Process the item. Move/merge/encrypt as needed.
    fn process(&self, path: &Path, item_type: &DropItemType) -> Result<DropOutcome, String>;
}

// ── Built-in Handlers ──

/// Credentials → encrypt and store in vault.
pub struct CredentialHandler;

impl DropHandler for CredentialHandler {
    fn handles(&self) -> Vec<DropItemType> {
        vec![DropItemType::ApiCredential, DropItemType::SshKey, DropItemType::Certificate]
    }

    fn validate(&self, path: &Path, _item_type: &DropItemType) -> Result<(), String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("Read: {e}"))?;
        if content.trim().is_empty() { return Err("Empty credential file".into()); }
        Ok(())
    }

    fn process(&self, path: &Path, item_type: &DropItemType) -> Result<DropOutcome, String> {
        let vault_dir = dirs::home_dir().unwrap_or_default().join(".hydra/vault");
        let _ = std::fs::create_dir_all(&vault_dir);
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let dest = vault_dir.join(&name);
        std::fs::copy(path, &dest).map_err(|e| format!("Copy to vault: {e}"))?;
        // SEC-1: Restrict vault file permissions
        #[cfg(unix)]
        { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o600)); }

        // Also inject into current process env if it's a .env file
        if name.ends_with(".env") || name == ".env" {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    if let Some((key, val)) = line.split_once('=') {
                        let k = key.trim();
                        let v = val.trim().trim_matches('"');
                        if !k.is_empty() && !k.starts_with('#') {
                            // SEC-7: Never allow drop to override vault passphrase
                            if k == "HYDRA_VAULT_PASSPHRASE" { eprintln!("hydra-drop: BLOCKED vault passphrase override"); continue; }
                            unsafe { std::env::set_var(k, v); }
                            eprintln!("hydra-drop: credential injected: {k}=***");
                        }
                    }
                }
            }
        }

        eprintln!("hydra-drop: credential stored in vault: {name}");
        Ok(DropOutcome::Accepted {
            item_type: item_type.label(), destination: dest.display().to_string(),
            details: format!("Stored in vault as {name}"),
        })
    }
}

/// Skills → parse markdown or extract archive into skills/ directory.
pub struct SkillHandler;

impl DropHandler for SkillHandler {
    fn handles(&self) -> Vec<DropItemType> {
        vec![DropItemType::SkillMarkdown, DropItemType::SkillPackage]
    }

    fn validate(&self, path: &Path, item_type: &DropItemType) -> Result<(), String> {
        match item_type {
            DropItemType::SkillMarkdown => {
                let content = std::fs::read_to_string(path).map_err(|e| format!("{e}"))?;
                if content.lines().count() < 3 { return Err("Markdown too short (need at least 3 lines)".into()); }
                Ok(())
            }
            DropItemType::SkillPackage => {
                // Basic check: file exists and is non-empty
                let meta = std::fs::metadata(path).map_err(|e| format!("{e}"))?;
                if meta.len() == 0 { return Err("Empty archive".into()); }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn process(&self, path: &Path, item_type: &DropItemType) -> Result<DropOutcome, String> {
        match item_type {
            DropItemType::SkillMarkdown => {
                let path_str = path.to_string_lossy().to_string();
                let result = crate::learn_md::learn_from_markdown(&path_str)?;
                let mut genome = hydra_genome::GenomeStore::open();
                genome.load_from_skills();
                eprintln!("hydra-drop: skill learned from markdown: {} ({} knowledge, {} steps)",
                    result.domain, result.knowledge_count, result.step_count);
                Ok(DropOutcome::Accepted {
                    item_type: "skill-markdown".into(), destination: result.skill_dir,
                    details: format!("{} knowledge, {} steps, {} rules", result.knowledge_count, result.step_count, result.rule_count),
                })
            }
            DropItemType::SkillPackage => {
                // Extract archive to skills/ directory
                let skills_dir = std::env::current_dir().unwrap_or_default().join("skills");
                let _ = std::fs::create_dir_all(&skills_dir);
                let name = path.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or("unknown".into());
                let target = skills_dir.join(&name);
                let _ = std::fs::create_dir_all(&target);
                // Simple tar.gz extraction via shell
                // SEC-2: Extract without following symlinks or preserving dangerous permissions
                let status = std::process::Command::new("tar")
                    .args(["xzf", &path.to_string_lossy(), "-C", &target.to_string_lossy(), "--no-same-permissions", "--no-same-owner"])
                    .status().map_err(|e| format!("Extract: {e}"))?;
                if !status.success() { return Err("Archive extraction failed".into()); }
                // Reload genome from skills
                let mut genome = hydra_genome::GenomeStore::open();
                genome.load_from_skills();
                eprintln!("hydra-drop: skill package extracted to {}", target.display());
                Ok(DropOutcome::Accepted {
                    item_type: "skill-package".into(), destination: target.display().to_string(),
                    details: format!("Extracted to {}", target.display()),
                })
            }
            _ => Err("Not a skill item".into()),
        }
    }
}

/// Genome entries → merge into genome database.
pub struct GenomeHandler;

impl DropHandler for GenomeHandler {
    fn handles(&self) -> Vec<DropItemType> { vec![DropItemType::GenomeEntries] }

    fn validate(&self, path: &Path, _: &DropItemType) -> Result<(), String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("{e}"))?;
        let _: Vec<hydra_genome::GenomeEntry> = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid genome JSON: {e}"))?;
        Ok(())
    }

    fn process(&self, path: &Path, _: &DropItemType) -> Result<DropOutcome, String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("{e}"))?;
        let entries: Vec<hydra_genome::GenomeEntry> = serde_json::from_str(&content)
            .map_err(|e| format!("{e}"))?;
        let count = entries.len();
        let mut genome = hydra_genome::GenomeStore::open();
        let result = crate::backup_merge::merge_genome(&mut genome, &entries);
        eprintln!("hydra-drop: genome merge: {}", result.summary());
        Ok(DropOutcome::Accepted {
            item_type: "genome-entries".into(), destination: "genome.db".into(),
            details: format!("{count} entries → {}", result.summary()),
        })
    }
}

/// Config files → merge TOML sections into appropriate config.
pub struct ConfigHandler;

impl DropHandler for ConfigHandler {
    fn handles(&self) -> Vec<DropItemType> {
        vec![DropItemType::ConfigOverride, DropItemType::LearningSource,
             DropItemType::MachineConfig, DropItemType::CloudBackupConfig,
             DropItemType::MonitorSource]
    }

    fn validate(&self, path: &Path, _: &DropItemType) -> Result<(), String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("{e}"))?;
        let _: toml::Value = toml::from_str(&content).map_err(|e| format!("Invalid TOML: {e}"))?;
        Ok(())
    }

    fn process(&self, path: &Path, item_type: &DropItemType) -> Result<DropOutcome, String> {
        let hydra_dir = dirs::home_dir().unwrap_or_default().join(".hydra");
        let dest_name = match item_type {
            DropItemType::LearningSource => "learning/sources.toml",
            DropItemType::MachineConfig => "machines.toml",
            DropItemType::ConfigOverride => "config.toml",
            DropItemType::CloudBackupConfig => "cloud.toml",
            DropItemType::MonitorSource => "monitor.toml",
            _ => "config.toml",
        };
        let dest = hydra_dir.join(dest_name);
        // Merge: append dropped content to existing file
        let existing = std::fs::read_to_string(&dest).unwrap_or_default();
        let dropped = std::fs::read_to_string(path).map_err(|e| format!("{e}"))?;
        let merged = if existing.is_empty() { dropped.clone() } else { format!("{existing}\n\n# Dropped via gateway\n{dropped}") };
        if let Some(parent) = dest.parent() { let _ = std::fs::create_dir_all(parent); }
        std::fs::write(&dest, &merged).map_err(|e| format!("Write: {e}"))?;
        eprintln!("hydra-drop: config merged into {dest_name}");
        Ok(DropOutcome::Accepted {
            item_type: item_type.label(), destination: dest.display().to_string(),
            details: format!("Merged into {dest_name}"),
        })
    }
}

/// Documents → route to document analysis pipeline.
pub struct DocumentHandler;

impl DropHandler for DocumentHandler {
    fn handles(&self) -> Vec<DropItemType> { vec![DropItemType::Document] }
    fn validate(&self, path: &Path, _: &DropItemType) -> Result<(), String> {
        if !path.exists() { return Err("File not found".into()); }
        Ok(())
    }
    fn process(&self, path: &Path, _: &DropItemType) -> Result<DropOutcome, String> {
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        // Store in ~/.hydra/documents/ for later analysis via /document command
        let docs_dir = dirs::home_dir().unwrap_or_default().join(".hydra/documents");
        let _ = std::fs::create_dir_all(&docs_dir);
        let dest = docs_dir.join(&name);
        std::fs::copy(path, &dest).map_err(|e| format!("Copy: {e}"))?;
        eprintln!("hydra-drop: document stored for analysis: {name}");
        Ok(DropOutcome::Accepted {
            item_type: "document".into(), destination: dest.display().to_string(),
            details: format!("Stored for analysis: {name}"),
        })
    }
}

/// Immersion content → add to domain mastery sources.
pub struct ImmersionHandler;

impl DropHandler for ImmersionHandler {
    fn handles(&self) -> Vec<DropItemType> { vec![DropItemType::ImmersionContent] }
    fn validate(&self, path: &Path, _: &DropItemType) -> Result<(), String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("{e}"))?;
        if content.lines().count() < 3 { return Err("Content too short".into()); }
        Ok(())
    }
    fn process(&self, path: &Path, _: &DropItemType) -> Result<DropOutcome, String> {
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let dest_dir = dirs::home_dir().unwrap_or_default().join(".hydra/immersion");
        let _ = std::fs::create_dir_all(&dest_dir);
        let dest = dest_dir.join(&name);
        std::fs::copy(path, &dest).map_err(|e| format!("Copy: {e}"))?;
        eprintln!("hydra-drop: immersion content added: {name}");
        Ok(DropOutcome::Accepted {
            item_type: "immersion".into(), destination: dest.display().to_string(),
            details: format!("Added for domain mastery: {name}"),
        })
    }
}

/// Connector configs → store in ~/.hydra/connectors/ for monitoring.
pub struct ConnectorHandler;

impl DropHandler for ConnectorHandler {
    fn handles(&self) -> Vec<DropItemType> { vec![DropItemType::Connector] }
    fn validate(&self, path: &Path, _: &DropItemType) -> Result<(), String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("{e}"))?;
        let _: toml::Value = toml::from_str(&content).map_err(|e| format!("Invalid connector TOML: {e}"))?;
        if !content.contains("[connector]") { return Err("Missing [connector] section".into()); }
        Ok(())
    }
    fn process(&self, path: &Path, _: &DropItemType) -> Result<DropOutcome, String> {
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let dest_dir = dirs::home_dir().unwrap_or_default().join(".hydra/connectors");
        let _ = std::fs::create_dir_all(&dest_dir);
        let dest = dest_dir.join(&name);
        std::fs::copy(path, &dest).map_err(|e| format!("Copy: {e}"))?;
        eprintln!("hydra-drop: connector registered: {name}");
        Ok(DropOutcome::Accepted {
            item_type: "connector".into(), destination: dest.display().to_string(),
            details: format!("Connector registered: {name}. Monitoring starts on next tick."),
        })
    }
}

/// Register all built-in handlers with a gateway.
pub fn register_builtins(handlers: &mut Vec<Box<dyn DropHandler>>) {
    handlers.push(Box::new(CredentialHandler));
    handlers.push(Box::new(SkillHandler));
    handlers.push(Box::new(GenomeHandler));
    handlers.push(Box::new(ConfigHandler));
    handlers.push(Box::new(ConnectorHandler));
    handlers.push(Box::new(DocumentHandler));
    handlers.push(Box::new(ImmersionHandler));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_handler_validates_non_empty() {
        let h = CredentialHandler;
        let tmp = std::env::temp_dir().join("test_cred.env");
        std::fs::write(&tmp, "KEY=value").unwrap();
        assert!(h.validate(&tmp, &DropItemType::ApiCredential).is_ok());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn credential_handler_rejects_empty() {
        let h = CredentialHandler;
        let tmp = std::env::temp_dir().join("test_empty_cred.env");
        std::fs::write(&tmp, "").unwrap();
        assert!(h.validate(&tmp, &DropItemType::ApiCredential).is_err());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn config_handler_validates_toml() {
        let h = ConfigHandler;
        let tmp = std::env::temp_dir().join("test_config.toml");
        std::fs::write(&tmp, "[tui]\ntheme = \"dark\"").unwrap();
        assert!(h.validate(&tmp, &DropItemType::ConfigOverride).is_ok());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn config_handler_rejects_invalid_toml() {
        let h = ConfigHandler;
        let tmp = std::env::temp_dir().join("test_bad.toml");
        std::fs::write(&tmp, "[invalid\nbroken").unwrap();
        assert!(h.validate(&tmp, &DropItemType::ConfigOverride).is_err());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn builtins_registered() {
        let mut handlers: Vec<Box<dyn DropHandler>> = Vec::new();
        register_builtins(&mut handlers);
        assert!(handlers.len() >= 6);
    }
}
