//! Local connector configuration — parsed from local.toml files.
//! Local connectors access filesystem, AppleScript, or local HTTP services
//! without internet. Direct access to the user's environment.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level local.toml structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConfig {
    pub integration: LocalIntegration,
    pub local: LocalSpec,
    #[serde(default)]
    pub credentials: LocalCredentials,
}

/// Integration metadata header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalIntegration {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_local_type")]
    pub r#type: String,
}

/// Local access specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSpec {
    /// Access method: filesystem, subprocess, http_local, applescript.
    pub access_method: String,
    /// Filesystem-specific settings.
    #[serde(default)]
    pub filesystem: Option<FilesystemSpec>,
    /// Local HTTP-specific settings.
    #[serde(default)]
    pub http_local: Option<HttpLocalSpec>,
    /// AppleScript-specific settings.
    #[serde(default)]
    pub applescript: Option<AppleScriptSpec>,
    /// Subprocess-specific settings.
    #[serde(default)]
    pub subprocess: Option<SubprocessSpec>,
    /// Capabilities (permissions).
    #[serde(default)]
    pub capabilities: LocalCapabilities,
    /// File system watcher settings.
    #[serde(default)]
    pub watch: LocalWatch,
}

/// Filesystem access config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemSpec {
    pub root_path: String,
    #[serde(default = "default_file_pattern")]
    pub file_pattern: String,
    #[serde(default = "default_true")]
    pub recursive: bool,
}

/// Local HTTP service config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpLocalSpec {
    pub base_url: String,
    #[serde(default)]
    pub discovery: Option<String>,
}

/// AppleScript config (macOS only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleScriptSpec {
    pub app_name: String,
    #[serde(default)]
    pub scripts: std::collections::HashMap<String, String>,
}

/// Subprocess command config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubprocessSpec {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Permission capabilities for this local connector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalCapabilities {
    #[serde(default = "default_true")]
    pub read: bool,
    #[serde(default)]
    pub write: bool,
    #[serde(default)]
    pub create: bool,
    #[serde(default)]
    pub delete: bool,
}

impl Default for LocalCapabilities {
    fn default() -> Self {
        Self {
            read: true,
            write: false,
            create: false,
            delete: false,
        }
    }
}

/// File system watcher configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalWatch {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_debounce")]
    pub debounce_ms: u64,
    #[serde(default)]
    pub events: Vec<String>,
}

impl Default for LocalWatch {
    fn default() -> Self {
        Self {
            enabled: false,
            debounce_ms: default_debounce(),
            events: vec![],
        }
    }
}

/// Credential configuration for local connectors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalCredentials {
    #[serde(default)]
    pub vault_service: Option<String>,
    #[serde(default)]
    pub auth_method: Option<String>,
}

// ── Defaults ──

fn default_local_type() -> String { "local".into() }
fn default_true() -> bool { true }
fn default_file_pattern() -> String { "*".into() }
fn default_debounce() -> u64 { 500 }

/// Load a local config from a TOML file.
pub fn load_local_config(path: &Path) -> Result<LocalConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    toml::from_str(&content)
        .map_err(|e| format!("parse {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_filesystem_local_toml() {
        let toml_str = r#"
[integration]
name = "obsidian"
description = "Obsidian vault access"

[local]
access_method = "filesystem"

[local.filesystem]
root_path = "~/Documents/Obsidian"
file_pattern = "*.md"

[local.capabilities]
read = true
write = true
create = true
delete = false

[local.watch]
enabled = true
debounce_ms = 500
events = ["create", "modify"]
"#;
        let config: LocalConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.integration.name, "obsidian");
        assert_eq!(config.local.access_method, "filesystem");
        let fs = config.local.filesystem.unwrap();
        assert_eq!(fs.file_pattern, "*.md");
        assert!(config.local.capabilities.read);
        assert!(config.local.capabilities.write);
        assert!(!config.local.capabilities.delete);
        assert!(config.local.watch.enabled);
    }

    #[test]
    fn parse_http_local_toml() {
        let toml_str = r#"
[integration]
name = "philips-hue"
description = "Smart lights"

[local]
access_method = "http_local"

[local.http_local]
base_url = "http://192.168.1.100/api"
discovery = "upnp"

[local.capabilities]
read = true
write = true

[credentials]
vault_service = "hue"
auth_method = "api_key"
"#;
        let config: LocalConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.local.access_method, "http_local");
        let http = config.local.http_local.unwrap();
        assert!(http.base_url.contains("192.168"));
        assert_eq!(config.credentials.vault_service, Some("hue".into()));
    }

    #[test]
    fn default_capabilities_read_only() {
        let caps = LocalCapabilities::default();
        assert!(caps.read);
        assert!(!caps.write);
        assert!(!caps.create);
        assert!(!caps.delete);
    }

    #[test]
    fn default_watch_disabled() {
        let watch = LocalWatch::default();
        assert!(!watch.enabled);
        assert_eq!(watch.debounce_ms, 500);
    }
}
