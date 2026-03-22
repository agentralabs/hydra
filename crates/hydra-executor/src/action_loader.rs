//! Action loader — reads action.toml files from actions/ folder.
//!
//! Each action defines what Hydra can DO:
//! shell commands, API calls, scheduled tasks.
//! Loaded on boot, no code changes needed.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A loaded action with its execution details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub description: String,
    pub trigger: TriggerType,
    pub approval: ApprovalMode,
    pub execute: ExecutionSpec,
    pub parameters: Vec<ActionParameter>,
    pub schedule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TriggerType {
    Manual,
    Scheduled,
    Conditional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApprovalMode {
    Required,
    Auto,
    Notify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSpec {
    pub exec_type: String,
    pub command: Option<String>,
    pub method: Option<String>,
    pub url: Option<String>,
    pub body: Option<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub default: Option<String>,
}

/// The action registry — all loaded actions.
#[derive(Debug, Default)]
pub struct ActionRegistry {
    actions: HashMap<String, Action>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
        }
    }

    /// Load all actions from the actions/ directory.
    pub fn load_from_directory(dir: &Path) -> Self {
        let mut registry = Self::new();

        if !dir.exists() {
            return registry;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("hydra: actions dir read failed: {e}");
                return registry;
            }
        };

        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let action_path = entry.path().join("action.toml");
            if !action_path.exists() {
                continue;
            }
            match load_action(&action_path) {
                Ok(action) => {
                    let name = action.name.clone();
                    eprintln!(
                        "hydra: action '{}' loaded (trigger={:?}, approval={:?})",
                        name, action.trigger, action.approval
                    );
                    registry.actions.insert(name, action);
                }
                Err(e) => {
                    eprintln!(
                        "hydra: action load failed for {:?}: {e}",
                        entry.path()
                    );
                }
            }
        }

        registry
    }

    pub fn get(&self, name: &str) -> Option<&Action> {
        self.actions.get(name)
    }

    pub fn list(&self) -> Vec<&str> {
        self.actions.keys().map(|s| s.as_str()).collect()
    }

    pub fn count(&self) -> usize {
        self.actions.len()
    }

    pub fn scheduled(&self) -> Vec<&Action> {
        self.actions
            .values()
            .filter(|a| a.trigger == TriggerType::Scheduled)
            .collect()
    }

    pub fn requires_approval(&self, name: &str) -> bool {
        self.actions
            .get(name)
            .map(|a| a.approval == ApprovalMode::Required)
            .unwrap_or(true) // default: require approval
    }
}

/// Raw TOML structures for parsing.
#[derive(Deserialize)]
struct ActionToml {
    action: ActionMeta,
}

#[derive(Deserialize)]
struct ActionMeta {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_trigger")]
    trigger: String,
    #[serde(default = "default_approval")]
    approval: String,
    #[serde(default)]
    schedule: Option<String>,
    execute: ExecuteBlock,
    #[serde(default)]
    parameters: Vec<ParamBlock>,
}

#[derive(Deserialize)]
struct ExecuteBlock {
    #[serde(rename = "type")]
    exec_type: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default = "default_timeout")]
    timeout_seconds: u64,
}

#[derive(Deserialize)]
struct ParamBlock {
    name: String,
    #[serde(rename = "type", default = "default_string")]
    param_type: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    default: Option<String>,
}

fn default_trigger() -> String {
    "manual".into()
}
fn default_approval() -> String {
    "required".into()
}
fn default_timeout() -> u64 {
    60
}
fn default_string() -> String {
    "string".into()
}

fn parse_trigger(s: &str) -> TriggerType {
    match s {
        "scheduled" => TriggerType::Scheduled,
        "conditional" => TriggerType::Conditional,
        _ => TriggerType::Manual,
    }
}

fn parse_approval(s: &str) -> ApprovalMode {
    match s {
        "auto" => ApprovalMode::Auto,
        "notify" => ApprovalMode::Notify,
        _ => ApprovalMode::Required,
    }
}

fn load_action(path: &Path) -> Result<Action, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let parsed: ActionToml =
        toml::from_str(&content).map_err(|e| format!("parse {}: {e}", path.display()))?;

    let meta = parsed.action;
    Ok(Action {
        name: meta.name,
        description: meta.description,
        trigger: parse_trigger(&meta.trigger),
        approval: parse_approval(&meta.approval),
        schedule: meta.schedule,
        execute: ExecutionSpec {
            exec_type: meta.execute.exec_type,
            command: meta.execute.command,
            method: meta.execute.method,
            url: meta.execute.url,
            body: meta.execute.body,
            timeout_seconds: meta.execute.timeout_seconds,
        },
        parameters: meta
            .parameters
            .into_iter()
            .map(|p| ActionParameter {
                name: p.name,
                param_type: p.param_type,
                required: p.required,
                default: p.default,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry() {
        let reg = ActionRegistry::new();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn load_nonexistent_dir() {
        let reg = ActionRegistry::load_from_directory(Path::new("/nonexistent"));
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn default_approval_is_required() {
        let reg = ActionRegistry::new();
        assert!(reg.requires_approval("anything"));
    }
}
