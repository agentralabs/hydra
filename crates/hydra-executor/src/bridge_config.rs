//! Bridge connector configuration — parsed from bridge.toml files.
//! Bridges are persistent subprocesses that relay messages to/from
//! external messaging platforms (WhatsApp, Telegram, Discord, etc.).

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level bridge.toml structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub integration: BridgeIntegration,
    pub bridge: BridgeSpec,
    #[serde(default)]
    pub credentials: BridgeCredentials,
}

/// Integration metadata header (shared with api.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeIntegration {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_bridge_type")]
    pub r#type: String,
}

/// Bridge subprocess specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeSpec {
    /// Runtime: node, python, binary, script.
    pub runtime: String,
    /// Entry point script/binary, relative to integration dir.
    pub entry: String,
    /// Communication transport: stdio, websocket, unix_socket.
    #[serde(default = "default_transport")]
    pub transport: String,
    /// Start bridge automatically on Hydra boot.
    #[serde(default = "default_true")]
    pub auto_start: bool,
    /// Restart bridge if its process dies.
    #[serde(default = "default_true")]
    pub restart_on_crash: bool,
    /// Health check interval in seconds.
    #[serde(default = "default_health_interval")]
    pub health_check_interval_seconds: u64,
    /// Maximum restart attempts before giving up.
    #[serde(default = "default_max_restarts")]
    pub max_restart_attempts: u32,
    /// Startup timeout in seconds.
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout_seconds: u64,
    /// Extra CLI args for the subprocess.
    #[serde(default)]
    pub args: Vec<String>,
    /// Incoming message config.
    #[serde(default)]
    pub incoming: BridgeIncoming,
    /// Outgoing message config.
    #[serde(default)]
    pub outgoing: BridgeOutgoing,
    /// Lifecycle commands.
    #[serde(default)]
    pub lifecycle: BridgeLifecycle,
}

/// How to parse incoming messages from the bridge subprocess.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeIncoming {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_message_field")]
    pub message_field: String,
    #[serde(default = "default_sender_field")]
    pub sender_field: String,
    #[serde(default = "default_timestamp_field")]
    pub timestamp_field: String,
    #[serde(default)]
    pub media_field: Option<String>,
}

impl Default for BridgeIncoming {
    fn default() -> Self {
        Self {
            format: default_format(),
            message_field: default_message_field(),
            sender_field: default_sender_field(),
            timestamp_field: default_timestamp_field(),
            media_field: None,
        }
    }
}

/// How to format outgoing messages to the bridge subprocess.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeOutgoing {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_message_field")]
    pub message_field: String,
    #[serde(default = "default_recipient_field")]
    pub recipient_field: String,
    #[serde(default)]
    pub media_field: Option<String>,
}

impl Default for BridgeOutgoing {
    fn default() -> Self {
        Self {
            format: default_format(),
            message_field: default_message_field(),
            recipient_field: default_recipient_field(),
            media_field: None,
        }
    }
}

/// Lifecycle commands sent to the bridge process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeLifecycle {
    #[serde(default = "default_init_cmd")]
    pub init_command: String,
    #[serde(default = "default_shutdown_cmd")]
    pub shutdown_command: String,
    #[serde(default = "default_health_cmd")]
    pub health_command: String,
    #[serde(default = "default_health_resp")]
    pub health_response: String,
}

impl Default for BridgeLifecycle {
    fn default() -> Self {
        Self {
            init_command: default_init_cmd(),
            shutdown_command: default_shutdown_cmd(),
            health_command: default_health_cmd(),
            health_response: default_health_resp(),
        }
    }
}

/// Credential configuration for the bridge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BridgeCredentials {
    #[serde(default)]
    pub vault_service: Option<String>,
    #[serde(default)]
    pub env_vars: Vec<String>,
    #[serde(default)]
    pub session_persist: bool,
    #[serde(default)]
    pub session_path: Option<String>,
}

// ── Defaults ──

fn default_bridge_type() -> String { "bridge".into() }
fn default_transport() -> String { "stdio".into() }
fn default_true() -> bool { true }
fn default_health_interval() -> u64 { 30 }
fn default_max_restarts() -> u32 { 5 }
fn default_startup_timeout() -> u64 { 10 }
fn default_format() -> String { "json_lines".into() }
fn default_message_field() -> String { "text".into() }
fn default_sender_field() -> String { "from".into() }
fn default_timestamp_field() -> String { "timestamp".into() }
fn default_recipient_field() -> String { "to".into() }
fn default_init_cmd() -> String { r#"{"type":"init"}"#.into() }
fn default_shutdown_cmd() -> String { r#"{"type":"shutdown"}"#.into() }
fn default_health_cmd() -> String { r#"{"type":"ping"}"#.into() }
fn default_health_resp() -> String { r#"{"type":"pong"}"#.into() }

/// Load a bridge config from a TOML file.
pub fn load_bridge_config(path: &Path) -> Result<BridgeConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    toml::from_str(&content)
        .map_err(|e| format!("parse {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_bridge_toml() {
        let toml_str = r#"
[integration]
name = "test-bridge"

[bridge]
runtime = "node"
entry = "bridge.js"
"#;
        let config: BridgeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.integration.name, "test-bridge");
        assert_eq!(config.bridge.runtime, "node");
        assert_eq!(config.bridge.transport, "stdio");
        assert!(config.bridge.auto_start);
        assert!(config.bridge.restart_on_crash);
        assert_eq!(config.bridge.health_check_interval_seconds, 30);
    }

    #[test]
    fn parse_full_bridge_toml() {
        let toml_str = r#"
[integration]
name = "telegram"
description = "Telegram Bot"
type = "bridge"

[bridge]
runtime = "node"
entry = "bridge.js"
transport = "stdio"
auto_start = false
restart_on_crash = true
max_restart_attempts = 3
args = ["--headless"]

[bridge.incoming]
message_field = "text"
sender_field = "sender"

[bridge.outgoing]
message_field = "message"
recipient_field = "chat_id"

[bridge.lifecycle]
init_command = '{"cmd":"start"}'
health_command = '{"cmd":"ping"}'
health_response = '{"cmd":"pong"}'

[credentials]
vault_service = "telegram"
env_vars = ["BOT_TOKEN"]
"#;
        let config: BridgeConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.bridge.auto_start);
        assert_eq!(config.bridge.max_restart_attempts, 3);
        assert_eq!(config.bridge.incoming.sender_field, "sender");
        assert_eq!(config.credentials.env_vars, vec!["BOT_TOKEN"]);
    }

    #[test]
    fn defaults_applied_correctly() {
        let config = BridgeConfig {
            integration: BridgeIntegration {
                name: "test".into(),
                description: String::new(),
                r#type: "bridge".into(),
            },
            bridge: BridgeSpec {
                runtime: "node".into(),
                entry: "index.js".into(),
                transport: default_transport(),
                auto_start: true,
                restart_on_crash: true,
                health_check_interval_seconds: 30,
                max_restart_attempts: 5,
                startup_timeout_seconds: 10,
                args: vec![],
                incoming: BridgeIncoming::default(),
                outgoing: BridgeOutgoing::default(),
                lifecycle: BridgeLifecycle::default(),
            },
            credentials: BridgeCredentials::default(),
        };
        assert_eq!(config.bridge.lifecycle.health_command, r#"{"type":"ping"}"#);
        assert_eq!(config.bridge.incoming.message_field, "text");
    }
}
