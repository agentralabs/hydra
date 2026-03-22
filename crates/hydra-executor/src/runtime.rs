//! Runtime executor — actually runs actions and integration calls.
//!
//! The loaders parse TOML. This module executes what they loaded.
//! Shell commands via std::process::Command.
//! API calls via reqwest.
//! Every execution is receipted.

use crate::action_loader::Action;
use crate::integration::{Capability, Integration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// Result of executing an action or integration call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub name: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub receipt_id: String,
}

/// Read credentials from vault for a given service.
pub fn read_credentials(service: &str) -> HashMap<String, String> {
    let vault_path = Path::new("vault").join(format!("{service}.toml"));
    if !vault_path.exists() {
        return HashMap::new();
    }
    let content = match std::fs::read_to_string(&vault_path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    #[derive(Deserialize)]
    struct VaultFile {
        #[serde(default)]
        credentials: HashMap<String, String>,
    }

    match toml::from_str::<VaultFile>(&content) {
        Ok(v) => v.credentials,
        Err(_) => HashMap::new(),
    }
}

/// Check if a vault entry permits the given operation.
pub fn check_vault_permission(service: &str, operation: &str) -> bool {
    let vault_path = Path::new("vault").join(format!("{service}.toml"));
    if !vault_path.exists() {
        return false;
    }
    let content = match std::fs::read_to_string(&vault_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    #[derive(Deserialize, Default)]
    struct AccessBlock {
        #[serde(default = "default_true")]
        read: bool,
        #[serde(default)]
        write: bool,
        #[serde(default)]
        delete: bool,
        #[serde(default)]
        spend: bool,
    }
    fn default_true() -> bool {
        true
    }

    #[derive(Deserialize)]
    struct VaultFile {
        #[serde(default)]
        access: AccessBlock,
    }

    let vault: VaultFile = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return false,
    };

    match operation {
        "read" => vault.access.read,
        "write" => vault.access.write,
        "delete" => vault.access.delete,
        "spend" => vault.access.spend,
        _ => vault.access.read,
    }
}

/// Execute a shell command from an action.
pub fn execute_shell(
    action: &Action,
    params: &HashMap<String, String>,
) -> ExecutionResult {
    let start = Instant::now();
    let receipt_id = uuid::Uuid::new_v4().to_string();

    let command = match &action.execute.command {
        Some(cmd) => substitute_params(cmd, params),
        None => {
            return ExecutionResult {
                name: action.name.clone(),
                success: false,
                output: String::new(),
                error: Some("No command specified".into()),
                duration_ms: 0,
                receipt_id,
            };
        }
    };

    eprintln!("hydra: executing action '{}': shell command", action.name);

    let result = std::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output();

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let success = output.status.success();

            eprintln!(
                "hydra: action '{}' completed (success={}, {}ms). Receipt: {}",
                action.name, success, duration_ms, &receipt_id[..8]
            );

            let error_msg = if success { None } else { Some(stderr.clone()) };
            ExecutionResult {
                name: action.name.clone(),
                success,
                output: if stdout.is_empty() { stderr } else { stdout },
                error: error_msg,
                duration_ms,
                receipt_id,
            }
        }
        Err(e) => ExecutionResult {
            name: action.name.clone(),
            success: false,
            output: String::new(),
            error: Some(e.to_string()),
            duration_ms,
            receipt_id,
        },
    }
}

/// Execute an API call from an integration capability.
pub fn execute_api_sync(
    integration: &Integration,
    capability: &Capability,
    params: &HashMap<String, String>,
) -> ExecutionResult {
    let start = Instant::now();
    let receipt_id = uuid::Uuid::new_v4().to_string();

    // Read credentials from vault
    let creds = read_credentials(&integration.name);
    if creds.is_empty() {
        return ExecutionResult {
            name: format!("{}.{}", integration.name, capability.name),
            success: false,
            output: String::new(),
            error: Some(format!(
                "No credentials found in vault/{}.toml",
                integration.name
            )),
            duration_ms: 0,
            receipt_id,
        };
    }

    // Build URL
    let endpoint = substitute_params(&capability.endpoint, params);
    let url = format!("{}{}", integration.base_url, endpoint);

    eprintln!(
        "hydra: executing integration '{}.{}': {} {}",
        integration.name, capability.name, capability.method, url
    );

    // Build and execute the request synchronously
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            return ExecutionResult {
                name: format!("{}.{}", integration.name, capability.name),
                success: false,
                output: String::new(),
                error: Some(format!("HTTP client error: {e}")),
                duration_ms: start.elapsed().as_millis() as u64,
                receipt_id,
            };
        }
    };

    let api_key = creds
        .get("api_key")
        .or_else(|| creds.get("token"))
        .cloned()
        .unwrap_or_default();

    let mut request = match capability.method.as_str() {
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        _ => client.get(&url),
    };

    // Add authentication
    match integration.auth_type.as_str() {
        "bearer" => {
            request = request.header("Authorization", format!("Bearer {api_key}"));
        }
        "header" => {
            if let Some(header_name) = &integration.auth_header {
                request = request.header(header_name.as_str(), &api_key);
            }
        }
        "query_param" => {
            if let Some(param_name) = &integration.auth_header {
                request = request.query(&[(param_name.as_str(), api_key.as_str())]);
            }
        }
        _ => {}
    }

    // Add body for POST/PUT
    if let Some(body_template) = &capability.body_template {
        let body = substitute_params(body_template, params);
        request = request
            .header("Content-Type", "application/json")
            .body(body);
    }

    let response = request.send();
    let duration_ms = start.elapsed().as_millis() as u64;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            let success = status.is_success();

            eprintln!(
                "hydra: integration '{}.{}' completed (status={}, {}ms). Receipt: {}",
                integration.name, capability.name, status, duration_ms, &receipt_id[..8]
            );

            ExecutionResult {
                name: format!("{}.{}", integration.name, capability.name),
                success,
                output: body,
                error: if success {
                    None
                } else {
                    Some(format!("HTTP {status}"))
                },
                duration_ms,
                receipt_id,
            }
        }
        Err(e) => ExecutionResult {
            name: format!("{}.{}", integration.name, capability.name),
            success: false,
            output: String::new(),
            error: Some(e.to_string()),
            duration_ms,
            receipt_id,
        },
    }
}

/// Substitute {param} placeholders in a string.
fn substitute_params(template: &str, params: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in params {
        result = result.replace(&format!("{{{key}}}"), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitute_params_works() {
        let mut params = HashMap::new();
        params.insert("name".to_string(), "world".to_string());
        params.insert("greeting".to_string(), "hello".to_string());
        let result = substitute_params("{greeting} {name}!", &params);
        assert_eq!(result, "hello world!");
    }

    #[test]
    fn empty_vault_returns_empty() {
        let creds = read_credentials("nonexistent-service");
        assert!(creds.is_empty());
    }

    #[test]
    fn vault_permission_default_deny() {
        assert!(!check_vault_permission("nonexistent", "write"));
    }
}
