//! Agent spawner — decomposes complex tasks into parallel sub-agents.
//!
//! Uses CollabManager to create collaboration sessions where multiple
//! "agents" (cognitive loop instances) work on subtasks concurrently.

use std::sync::Arc;
use parking_lot::Mutex;
use hydra_collab::CollabManager;

/// Task that can be delegated to a sub-agent.
#[derive(Debug, Clone)]
pub struct SubTask {
    pub id: String,
    pub description: String,
    /// Module domain, e.g. "auth", "catalog", "search".
    pub module: String,
}

/// Result from a sub-agent.
#[derive(Debug, Clone)]
pub struct SubTaskResult {
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub files_created: usize,
}

/// Agent spawner wrapping CollabManager for parallel sub-agent orchestration.
pub struct AgentSpawner {
    collab: Arc<Mutex<CollabManager>>,
    max_agents: usize,
}

impl AgentSpawner {
    pub fn new(max_agents: usize) -> Self {
        Self {
            collab: Arc::new(Mutex::new(CollabManager::new())),
            max_agents: max_agents.min(100), // Hard cap at 100
        }
    }

    /// Decompose a complex task description into subtasks by detecting
    /// domain modules mentioned in the text.
    pub fn decompose(&self, task_description: &str) -> Vec<SubTask> {
        let lower = task_description.to_lowercase();
        let mut subtasks = Vec::new();

        // Split into words for whole-word matching to avoid false positives
        // (e.g. "build" should not match keyword "ui").
        let words: Vec<&str> = lower.split_whitespace().collect();

        let modules: &[(&str, &[&str])] = &[
            ("auth", &["auth", "login", "register", "jwt", "session", "password"]),
            ("catalog", &["product", "catalog", "item", "listing", "inventory"]),
            ("search", &["search", "find", "filter", "query", "index"]),
            ("cart", &["cart", "basket", "checkout", "order"]),
            ("payment", &["payment", "pay", "stripe", "billing", "invoice"]),
            ("admin", &["admin", "dashboard", "manage", "analytics"]),
            ("ui", &["frontend", "ui", "component", "page", "layout", "style"]),
            ("api", &["api", "endpoint", "route", "controller", "middleware"]),
            ("database", &["database", "schema", "migration", "model", "seed"]),
            ("tests", &["test", "spec", "assert", "coverage"]),
        ];

        for (module_name, keywords) in modules {
            // Match whole words: a word must either equal or start with the keyword
            // (to handle plurals like "products" matching "product").
            // Strip common punctuation from words for matching.
            if keywords.iter().any(|kw| {
                words.iter().any(|w| {
                    let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
                    clean == *kw || clean.starts_with(kw)
                })
            }) {
                let uid = uuid::Uuid::new_v4();
                let short = uid.to_string();
                let prefix = short.split('-').next().unwrap_or("0");
                subtasks.push(SubTask {
                    id: format!("subtask-{}-{}", module_name, prefix),
                    description: format!(
                        "Build {} module for: {}",
                        module_name,
                        &task_description[..task_description.len().min(100)]
                    ),
                    module: module_name.to_string(),
                });
            }
        }

        // If it looks like a full project build but no specific modules matched,
        // create a default set.
        if subtasks.is_empty()
            && (lower.contains("build")
                || lower.contains("create")
                || lower.contains("website")
                || lower.contains("app"))
        {
            for module in &["api", "ui", "database", "tests"] {
                let uid = uuid::Uuid::new_v4();
                let short = uid.to_string();
                let prefix = short.split('-').next().unwrap_or("0");
                subtasks.push(SubTask {
                    id: format!("subtask-{}-{}", module, prefix),
                    description: format!(
                        "Build {} module for: {}",
                        module,
                        &task_description[..task_description.len().min(100)]
                    ),
                    module: module.to_string(),
                });
            }
        }

        // Cap at max_agents
        subtasks.truncate(self.max_agents);
        subtasks
    }

    /// Create a collaboration session for the given subtasks.
    /// Returns the session ID.
    pub fn create_session(&self, _task_description: &str, subtasks: &[SubTask]) -> String {
        let session_id = format!("session-{}", uuid::Uuid::new_v4());
        let agent_ids: Vec<String> = subtasks.iter().map(|t| t.id.clone()).collect();

        let mut collab = self.collab.lock();
        collab.create_session(&session_id, agent_ids);
        collab.activate(&session_id);

        session_id
    }

    /// Complete a collaboration session.
    pub fn complete_session(&self, session_id: &str) {
        let mut collab = self.collab.lock();
        collab.complete(session_id);
    }

    /// Check if a task is complex enough to warrant spawning (>= 2 subtasks).
    pub fn should_spawn(&self, task_description: &str) -> bool {
        let subtasks = self.decompose(task_description);
        subtasks.len() >= 2
    }

    /// Get active session count.
    pub fn active_sessions(&self) -> usize {
        let collab = self.collab.lock();
        collab.active_sessions().len()
    }

    /// Maximum parallel agents.
    pub fn max_agents(&self) -> usize {
        self.max_agents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose_ecommerce() {
        let spawner = AgentSpawner::new(10);
        let subtasks = spawner.decompose(
            "Build an e-commerce site with auth, product catalog, search, cart, and payment",
        );
        let modules: Vec<&str> = subtasks.iter().map(|s| s.module.as_str()).collect();
        assert!(modules.contains(&"auth"), "should detect auth module");
        assert!(modules.contains(&"catalog"), "should detect catalog module");
        assert!(modules.contains(&"search"), "should detect search module");
        assert!(modules.contains(&"cart"), "should detect cart module");
        assert!(modules.contains(&"payment"), "should detect payment module");
        assert!(subtasks.len() >= 5, "should have at least 5 subtasks");
    }

    #[test]
    fn test_decompose_generic_app() {
        let spawner = AgentSpawner::new(10);
        let subtasks = spawner.decompose("Build a cool new app");
        let modules: Vec<&str> = subtasks.iter().map(|s| s.module.as_str()).collect();
        // Generic "build ... app" triggers the default set
        assert!(modules.contains(&"api"));
        assert!(modules.contains(&"ui"));
        assert!(modules.contains(&"database"));
        assert!(modules.contains(&"tests"));
        assert_eq!(subtasks.len(), 4);
    }

    #[test]
    fn test_decompose_simple_query() {
        let spawner = AgentSpawner::new(10);
        let subtasks = spawner.decompose("What is the weather today?");
        assert!(subtasks.is_empty(), "simple queries should not decompose");
    }

    #[test]
    fn test_should_spawn() {
        let spawner = AgentSpawner::new(10);
        assert!(
            spawner.should_spawn("Build an app with auth and payment"),
            "should spawn for multi-module task"
        );
        assert!(
            !spawner.should_spawn("What is the weather today?"),
            "should not spawn for simple query"
        );
    }

    #[test]
    fn test_max_agents_cap() {
        // Even if you request 200, it caps at 100
        let spawner = AgentSpawner::new(200);
        assert_eq!(spawner.max_agents(), 100);

        // And truncate works: a task with many modules still caps
        let spawner_small = AgentSpawner::new(2);
        let subtasks = spawner_small.decompose(
            "Build a site with auth, product catalog, search, cart, payment, admin, ui, api, database, tests",
        );
        assert!(subtasks.len() <= 2, "should cap at max_agents=2");
    }

    #[test]
    fn test_session_lifecycle() {
        let spawner = AgentSpawner::new(10);
        let subtasks = spawner.decompose("Build an app with auth and payment");
        assert!(subtasks.len() >= 2);

        let session_id = spawner.create_session("Build an app with auth and payment", &subtasks);
        assert!(session_id.starts_with("session-"));
        assert_eq!(spawner.active_sessions(), 1);

        spawner.complete_session(&session_id);
        assert_eq!(spawner.active_sessions(), 0);
    }
}
