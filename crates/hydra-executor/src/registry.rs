//! ActionRegistry — stores action primitives from loaded skills.
//! Populated by hydra-skills when a skill loads.
//! Queried by the engine when executing.

use crate::constants::MAX_REGISTERED_ACTIONS;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How an action executes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutorType {
    /// Shell command with parameter substitution.
    Shell { command_template: String },
    /// Internal Rust function (built-in actions).
    Internal { handler: String },
    /// HTTP request to an endpoint.
    Http {
        method: String,
        url_template: String,
    },
    /// MCP tool call to a sister.
    Sister {
        sister_name: String,
        tool_name: String,
    },
}

/// Input parameter for an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionParam {
    pub name: String,
    pub required: bool,
    pub default: Option<String>,
}

/// One registered action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredAction {
    pub id: String,
    pub skill: String,
    pub description: String,
    pub verb: String,
    pub executor: ExecutorType,
    pub reversible: bool,
    pub estimated_ms: u64,
    pub input_params: Vec<ActionParam>,
}

/// The action registry — all actions from all loaded skills.
#[derive(Debug, Default)]
pub struct ActionRegistry {
    actions: HashMap<String, RegisteredAction>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register actions from a skill (called by hydra-skills on load).
    pub fn register_skill_actions(
        &mut self,
        skill_name: &str,
        actions: Vec<RegisteredAction>,
    ) -> usize {
        let mut registered = 0;
        for mut action in actions {
            if self.actions.len() >= MAX_REGISTERED_ACTIONS {
                break;
            }
            action.skill = skill_name.to_string();
            self.actions.insert(action.id.clone(), action);
            registered += 1;
        }
        registered
    }

    /// Unregister all actions from a skill (called on skill unload).
    pub fn unregister_skill(&mut self, skill_name: &str) -> usize {
        let before = self.actions.len();
        self.actions.retain(|_, v| v.skill != skill_name);
        before - self.actions.len()
    }

    pub fn get(&self, action_id: &str) -> Option<&RegisteredAction> {
        self.actions.get(action_id)
    }

    pub fn count(&self) -> usize {
        self.actions.len()
    }

    pub fn skills(&self) -> Vec<String> {
        let mut skills: Vec<String> = self
            .actions
            .values()
            .map(|a| a.skill.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        skills.sort();
        skills
    }

    /// Find actions by partial id or description match.
    pub fn search(&self, query: &str) -> Vec<&RegisteredAction> {
        let lower = query.to_lowercase();
        self.actions
            .values()
            .filter(|a| {
                a.id.contains(&lower)
                    || a.description.to_lowercase().contains(&lower)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_action(id: &str) -> RegisteredAction {
        RegisteredAction {
            id: id.to_string(),
            skill: "test-skill".to_string(),
            description: format!("Test action {}", id),
            verb: "testing".to_string(),
            executor: ExecutorType::Internal {
                handler: "test".to_string(),
            },
            reversible: false,
            estimated_ms: 100,
            input_params: vec![],
        }
    }

    #[test]
    fn register_and_retrieve() {
        let mut reg = ActionRegistry::new();
        reg.register_skill_actions(
            "test-skill",
            vec![make_action("test.action")],
        );
        assert!(reg.get("test.action").is_some());
        assert_eq!(reg.count(), 1);
    }

    #[test]
    fn unregister_removes_skill_actions() {
        let mut reg = ActionRegistry::new();
        reg.register_skill_actions(
            "skill-a",
            vec![make_action("a.one"), make_action("a.two")],
        );
        reg.register_skill_actions("skill-b", vec![make_action("b.one")]);
        assert_eq!(reg.count(), 3);
        let removed = reg.unregister_skill("skill-a");
        assert_eq!(removed, 2);
        assert_eq!(reg.count(), 1);
        assert!(reg.get("b.one").is_some());
    }

    #[test]
    fn search_finds_by_description() {
        let mut reg = ActionRegistry::new();
        reg.register_skill_actions(
            "skill",
            vec![make_action("deploy.staging")],
        );
        let results = reg.search("deploy");
        assert!(!results.is_empty());
    }
}
