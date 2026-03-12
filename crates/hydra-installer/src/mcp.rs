use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::error::InstallerError;

/// A single MCP server entry in the configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServerEntry {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

/// Top-level MCP configuration file structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct McpConfig {
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: BTreeMap<String, McpServerEntry>,
}

/// Merge two MCP configs. Never overwrites existing entries — only adds new ones.
pub fn merge_mcp_config(existing: &McpConfig, new: &McpConfig) -> McpConfig {
    let mut merged = existing.clone();
    for (name, entry) in &new.mcp_servers {
        // Only insert if the key does not already exist
        merged
            .mcp_servers
            .entry(name.clone())
            .or_insert_with(|| entry.clone());
    }
    merged
}

/// Load an MCP config from a JSON file on disk.
pub fn load_mcp_config(path: &Path) -> Result<McpConfig, InstallerError> {
    let contents = std::fs::read_to_string(path).map_err(|e| InstallerError::Io {
        context: format!("reading MCP config from {}", path.display()),
        source: e,
    })?;
    let config: McpConfig =
        serde_json::from_str(&contents).map_err(|e| InstallerError::ConfigParse {
            path: path.to_path_buf(),
            source: e,
        })?;
    Ok(config)
}

/// Save an MCP config to a JSON file on disk (pretty-printed).
pub fn save_mcp_config(path: &Path, config: &McpConfig) -> Result<(), InstallerError> {
    let json =
        serde_json::to_string_pretty(config).map_err(|e| InstallerError::ConfigParse {
            path: path.to_path_buf(),
            source: e,
        })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| InstallerError::Io {
            context: format!("creating directory {}", parent.display()),
            source: e,
        })?;
    }
    std::fs::write(path, json).map_err(|e| InstallerError::Io {
        context: format!("writing MCP config to {}", path.display()),
        source: e,
    })?;
    Ok(())
}
