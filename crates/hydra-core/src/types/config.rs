use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::kernel::RiskLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydraConfig {
    pub core: CoreConfig,
    pub sisters: SistersConfig,
    pub execution: ExecutionConfig,
    pub security: SecurityConfig,
    pub voice: Option<VoiceConfig>,
    pub server: Option<ServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub data_dir: PathBuf,
    pub log_level: String,
    pub token_budget: u64,
    pub max_concurrent_deployments: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SistersConfig {
    pub auto_discover: bool,
    pub connections: Vec<SisterEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SisterEndpoint {
    pub name: String,
    pub endpoint: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub require_approval_above: RiskLevel,
    pub max_retries: usize,
    pub timeout_seconds: u64,
    pub sandbox_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_signing: bool,
    pub enable_encryption: bool,
    pub allowed_shell_commands: Vec<String>,
    pub blocked_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub enabled: bool,
    pub wake_word: String,
    pub model_path: Option<PathBuf>,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
    pub enable_websocket: bool,
}

impl Default for HydraConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig {
                data_dir: dirs_default(),
                log_level: "info".to_string(),
                token_budget: 100_000,
                max_concurrent_deployments: 4,
            },
            sisters: SistersConfig {
                auto_discover: true,
                connections: vec![],
            },
            execution: ExecutionConfig {
                require_approval_above: RiskLevel::Medium,
                max_retries: 3,
                timeout_seconds: 300,
                sandbox_mode: true,
            },
            security: SecurityConfig {
                enable_signing: true,
                enable_encryption: false,
                allowed_shell_commands: vec![],
                blocked_paths: vec![],
            },
            voice: None,
            server: None,
        }
    }
}

fn dirs_default() -> PathBuf {
    dirs_home().join(".hydra")
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
