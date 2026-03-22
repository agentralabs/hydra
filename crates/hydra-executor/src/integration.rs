//! Integration loader — reads api.toml files from integrations/ folder.
//!
//! Each integration defines what Hydra can connect to:
//! endpoints, auth type, read capabilities, write capabilities.
//! Loaded on boot, no code changes needed.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A loaded integration with all its capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    pub name: String,
    pub description: String,
    pub protocol: String,
    pub base_url: String,
    pub auth_type: String,
    pub auth_header: Option<String>,
    pub read_capabilities: Vec<Capability>,
    pub write_capabilities: Vec<Capability>,
}

/// A single capability (read or write) of an integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub endpoint: String,
    pub method: String,
    pub description: String,
    pub requires_approval: bool,
    pub body_template: Option<String>,
}

/// The integration registry — all loaded integrations.
#[derive(Debug, Default)]
pub struct IntegrationRegistry {
    integrations: HashMap<String, Integration>,
}

impl IntegrationRegistry {
    pub fn new() -> Self {
        Self {
            integrations: HashMap::new(),
        }
    }

    /// Load all integrations from the integrations/ directory.
    pub fn load_from_directory(dir: &Path) -> Self {
        let mut registry = Self::new();

        if !dir.exists() {
            return registry;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("hydra: integrations dir read failed: {e}");
                return registry;
            }
        };

        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let api_path = entry.path().join("api.toml");
            if !api_path.exists() {
                continue;
            }
            match load_integration(&api_path) {
                Ok(integration) => {
                    let name = integration.name.clone();
                    let read_count = integration.read_capabilities.len();
                    let write_count = integration.write_capabilities.len();
                    eprintln!(
                        "hydra: integration '{}' loaded ({} read, {} write)",
                        name, read_count, write_count
                    );
                    registry.integrations.insert(name, integration);
                }
                Err(e) => {
                    eprintln!(
                        "hydra: integration load failed for {:?}: {e}",
                        entry.path()
                    );
                }
            }
        }

        registry
    }

    pub fn get(&self, name: &str) -> Option<&Integration> {
        self.integrations.get(name)
    }

    pub fn list(&self) -> Vec<&str> {
        self.integrations.keys().map(|s| s.as_str()).collect()
    }

    pub fn count(&self) -> usize {
        self.integrations.len()
    }

    pub fn find_capability(&self, action: &str) -> Option<(&Integration, &Capability)> {
        for integration in self.integrations.values() {
            for cap in &integration.read_capabilities {
                if cap.name == action {
                    return Some((integration, cap));
                }
            }
            for cap in &integration.write_capabilities {
                if cap.name == action {
                    return Some((integration, cap));
                }
            }
        }
        None
    }
}

/// Raw TOML structure for parsing api.toml.
#[derive(Deserialize)]
struct ApiToml {
    integration: IntegrationMeta,
    #[serde(default)]
    capabilities: CapabilitiesBlock,
}

#[derive(Deserialize)]
struct IntegrationMeta {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_protocol")]
    protocol: String,
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    auth_type: String,
    #[serde(default)]
    auth_header: Option<String>,
}

#[derive(Deserialize, Default)]
struct CapabilitiesBlock {
    #[serde(default)]
    read: Vec<RawCapability>,
    #[serde(default)]
    write: Vec<RawCapability>,
}

#[derive(Deserialize)]
struct RawCapability {
    name: String,
    #[serde(default)]
    endpoint: String,
    #[serde(default = "default_method")]
    method: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    requires_approval: bool,
    #[serde(default)]
    body_template: Option<String>,
}

fn default_protocol() -> String {
    "REST".into()
}
fn default_method() -> String {
    "GET".into()
}

fn load_integration(path: &Path) -> Result<Integration, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let parsed: ApiToml =
        toml::from_str(&content).map_err(|e| format!("parse {}: {e}", path.display()))?;

    Ok(Integration {
        name: parsed.integration.name,
        description: parsed.integration.description,
        protocol: parsed.integration.protocol,
        base_url: parsed.integration.base_url,
        auth_type: parsed.integration.auth_type,
        auth_header: parsed.integration.auth_header,
        read_capabilities: parsed
            .capabilities
            .read
            .into_iter()
            .map(|c| Capability {
                name: c.name,
                endpoint: c.endpoint,
                method: c.method,
                description: c.description,
                requires_approval: c.requires_approval,
                body_template: c.body_template,
            })
            .collect(),
        write_capabilities: parsed
            .capabilities
            .write
            .into_iter()
            .map(|c| Capability {
                name: c.name,
                endpoint: c.endpoint,
                method: c.method,
                description: c.description,
                requires_approval: true, // write always requires approval
                body_template: c.body_template,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry() {
        let reg = IntegrationRegistry::new();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn load_nonexistent_dir() {
        let reg = IntegrationRegistry::load_from_directory(Path::new("/nonexistent"));
        assert_eq!(reg.count(), 0);
    }
}
