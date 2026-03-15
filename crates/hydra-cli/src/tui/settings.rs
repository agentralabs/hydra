use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct HydraSettings {
    pub model: Option<String>,
    #[serde(rename = "maxTokens")]
    pub max_tokens: Option<u32>,
    pub permissions: PermissionSettings,
    pub hooks: HashMap<String, Vec<HookRule>>,
    #[serde(rename = "fastMode")]
    pub fast_mode: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PermissionSettings {
    #[serde(rename = "allowedTools")]
    pub allowed_tools: Vec<String>,
    pub deny: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct HookRule {
    pub matcher: String,
    pub hooks: Vec<HookAction>,
}

#[derive(Debug, Deserialize)]
pub struct HookAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub command: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct KeybindingsConfig {
    pub bindings: Vec<KeybindingContext>,
}

#[derive(Debug, Deserialize)]
pub struct KeybindingContext {
    pub context: String,
    pub bindings: HashMap<String, String>,
}

/// Load settings from project and global config files.
/// Project settings override global settings.
pub fn load_settings() -> HydraSettings {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();

    // Load global settings first
    let global_path = format!("{}/.hydra/settings.json", home);
    let mut settings = load_settings_file(&global_path).unwrap_or_default();

    // Override with project settings
    let project_path = ".hydra/settings.json";
    if let Some(project) = load_settings_file(project_path) {
        if project.model.is_some() {
            settings.model = project.model;
        }
        if project.max_tokens.is_some() {
            settings.max_tokens = project.max_tokens;
        }
        if !project.permissions.allowed_tools.is_empty() {
            settings.permissions.allowed_tools = project.permissions.allowed_tools;
        }
        if !project.permissions.deny.is_empty() {
            settings.permissions.deny = project.permissions.deny;
        }
        settings.hooks.extend(project.hooks);
        if project.fast_mode {
            settings.fast_mode = project.fast_mode;
        }
    }

    settings
}

fn load_settings_file(path: &str) -> Option<HydraSettings> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Load keybindings from ~/.hydra/keybindings.json
pub fn load_keybindings() -> KeybindingsConfig {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
    let path = format!("{}/.hydra/keybindings.json", home);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

/// Execute hooks matching an event and tool name.
pub fn execute_hooks(settings: &HydraSettings, event: &str, tool_name: &str) {
    if let Some(rules) = settings.hooks.get(event) {
        for rule in rules {
            if matches_tool(&rule.matcher, tool_name) {
                for action in &rule.hooks {
                    if action.action_type == "command" {
                        if let Some(ref cmd) = action.command {
                            let (shell, shell_arg) = hydra_native::utils::shell_command();
                            let _ = std::process::Command::new(shell)
                                .args([shell_arg, cmd.as_str()])
                                .output();
                        }
                    }
                }
            }
        }
    }
}

fn matches_tool(pattern: &str, tool: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.contains('*') {
        let prefix = pattern.trim_end_matches('*');
        tool.starts_with(prefix)
    } else {
        pattern == tool
    }
}
