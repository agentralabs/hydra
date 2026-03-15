//! Knowledge acquisition engine — learns about projects, APIs, and frameworks on-the-fly.
//!
//! When Hydra encounters an unfamiliar codebase or framework:
//! 1. DocReader finds and reads documentation files
//! 2. ApiLearner extracts API patterns via LLM
//! 3. SkillExtractor converts knowledge to stored beliefs
//!
//! All knowledge is persisted as beliefs for future sessions.

pub mod api_learner;
pub mod doc_reader;
pub mod skill_extractor;
// Cognitive Amplification modules
pub mod causal_model;
pub mod meta_reasoning;
pub mod compiled_reasoning;
pub mod sister_synapse;
pub mod reasoning_verifier;
pub mod awareness_mesh;
pub mod morning_briefing;
pub mod production_orchestrator;
pub mod remotion_bridge;
pub mod social_reasoning;
pub mod mentor_system;
pub mod creative_engine;
pub mod knowledge_hunter;
pub mod economics_tracker;
// Surgical edit system
pub mod edit_tool;
pub mod diff_engine;
pub mod change_tracker;

pub use api_learner::{ApiEndpoint, ApiKnowledge, FixKnowledge};
pub use doc_reader::{DocFile, DocKind};
pub use skill_extractor::ProjectKnowledge;

use std::path::Path;

/// Main knowledge acquisition coordinator.
///
/// Orchestrates doc reading, LLM-based learning, and belief storage.
/// Does NOT call the LLM itself — returns prompts and parses responses.
/// The caller (cognitive loop) handles actual LLM invocation.
pub struct KnowledgeAcquirer;

impl KnowledgeAcquirer {
    /// Create a new knowledge acquirer.
    pub fn new() -> Self {
        Self
    }

    /// Find documentation files in a project, ordered by relevance.
    pub fn find_docs(&self, project_root: &Path) -> Vec<DocFile> {
        doc_reader::find_docs(project_root)
    }

    /// Extract the most relevant content from a doc file.
    pub fn extract_doc(&self, doc: &DocFile, max_chars: usize) -> String {
        doc_reader::extract_relevant(doc, max_chars)
    }

    /// Build an LLM prompt to learn about a project from its README.
    /// Returns (prompt, project_name).
    pub fn build_readme_prompt(&self, readme_path: &Path) -> Option<(String, String)> {
        let content = std::fs::read_to_string(readme_path).ok()?;
        if content.is_empty() {
            return None;
        }
        let project_name = readme_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let prompt = skill_extractor::build_readme_learn_prompt(&content);
        Some((prompt, project_name))
    }

    /// Parse the LLM response for a README learning prompt.
    pub fn parse_readme_response(&self, project_name: &str, response: &str) -> ProjectKnowledge {
        ProjectKnowledge::parse_from_llm(project_name, response)
    }

    /// Build an LLM prompt to learn about an API from documentation.
    pub fn build_api_prompt(&self, doc_content: &str) -> String {
        api_learner::build_api_learn_prompt(doc_content)
    }

    /// Parse the LLM response for an API learning prompt.
    pub fn parse_api_response(&self, name: &str, response: &str) -> ApiKnowledge {
        ApiKnowledge::parse_from_llm(name, response)
    }

    /// Build an LLM prompt for fixing an error using documentation.
    pub fn build_error_fix_prompt(&self, error: &str, docs: &str) -> String {
        api_learner::build_error_fix_prompt(error, docs)
    }

    /// Parse the LLM response for an error fix prompt.
    pub fn parse_error_fix_response(&self, response: &str) -> Option<FixKnowledge> {
        FixKnowledge::parse_from_llm(response)
    }

    /// Convert project knowledge to belief tuples for storage.
    pub fn project_beliefs(&self, knowledge: &ProjectKnowledge) -> Vec<(String, String, String)> {
        skill_extractor::project_as_beliefs(knowledge)
    }

    /// Convert API knowledge to belief tuples for storage.
    pub fn api_beliefs(&self, knowledge: &ApiKnowledge) -> Vec<(String, String, String)> {
        skill_extractor::api_as_beliefs(knowledge)
    }

    /// Convert fix knowledge to a belief tuple for storage.
    pub fn fix_belief(&self, knowledge: &FixKnowledge) -> (String, String, String) {
        skill_extractor::fix_as_belief(knowledge)
    }

    /// Full learning flow for a project: find docs → extract → build prompts.
    /// Returns a list of (prompt, context_label) pairs for the caller to send to LLM.
    pub fn plan_learning(&self, project_root: &Path) -> Vec<(String, String)> {
        let docs = self.find_docs(project_root);
        let mut prompts = Vec::new();

        // Learn from README first
        if let Some(readme) = docs.iter().find(|d| d.is_readme()) {
            if let Some((prompt, name)) = self.build_readme_prompt(&readme.path) {
                prompts.push((prompt, format!("readme:{}", name)));
            }
        }

        // Learn from API docs
        for doc in docs.iter().filter(|d| d.kind == DocKind::ApiDocs) {
            let content = self.extract_doc(doc, 6000);
            if !content.is_empty() {
                let label = doc.path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("api");
                prompts.push((
                    self.build_api_prompt(&content),
                    format!("api:{}", label),
                ));
            }
        }

        prompts
    }
}

impl Default for KnowledgeAcquirer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_knowledge_acquirer_new() {
        let ka = KnowledgeAcquirer::new();
        let docs = ka.find_docs(Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap().parent().unwrap());
        assert!(!docs.is_empty());
    }

    #[test]
    fn test_plan_learning() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap().parent().unwrap();
        let ka = KnowledgeAcquirer::new();
        let prompts = ka.plan_learning(root);
        // Should generate at least a README prompt if one exists
        if root.join("README.md").exists() {
            assert!(!prompts.is_empty());
            assert!(prompts[0].1.starts_with("readme:"));
        }
    }

    #[test]
    fn test_build_readme_prompt_nonexistent() {
        let ka = KnowledgeAcquirer::new();
        assert!(ka.build_readme_prompt(Path::new("/nonexistent/README.md")).is_none());
    }

    #[test]
    fn test_parse_readme_response() {
        let ka = KnowledgeAcquirer::new();
        let response = r#"{"purpose": "test framework", "setup_commands": ["make"], "test_commands": ["make test"], "api_endpoints": [], "dependencies": []}"#;
        let k = ka.parse_readme_response("myproject", response);
        assert_eq!(k.purpose, "test framework");
        assert_eq!(k.project_name, "myproject");
    }

    #[test]
    fn test_parse_api_response() {
        let ka = KnowledgeAcquirer::new();
        let response = r#"{"endpoints": [{"method": "GET", "path": "/health", "description": "Health check", "params": []}], "key_types": [], "auth_method": null, "base_url": null}"#;
        let k = ka.parse_api_response("myapi", response);
        assert_eq!(k.endpoints.len(), 1);
    }

    #[test]
    fn test_error_fix_flow() {
        let ka = KnowledgeAcquirer::new();
        let prompt = ka.build_error_fix_prompt("command not found: jq", "Install jq with brew");
        assert!(prompt.contains("command not found"));

        let response = r#"{"error_pattern": "missing tool", "root_cause": "jq not installed", "fix_steps": ["brew install jq"], "confidence": 0.95}"#;
        let fix = ka.parse_error_fix_response(response).unwrap();
        assert_eq!(fix.fix_steps, vec!["brew install jq"]);

        let belief = ka.fix_belief(&fix);
        assert!(belief.0.contains("missing tool"));
    }

    #[test]
    fn test_project_beliefs_roundtrip() {
        let ka = KnowledgeAcquirer::new();
        let response = r#"{"purpose": "web app", "setup_commands": ["npm install"], "test_commands": ["npm test"], "api_endpoints": [], "dependencies": ["node 18"]}"#;
        let k = ka.parse_readme_response("webapp", response);
        let beliefs = ka.project_beliefs(&k);
        assert!(!beliefs.is_empty());
        assert!(beliefs.iter().any(|(s, _, _)| s.contains("purpose")));
    }

    #[test]
    fn test_default_trait() {
        let _ka = KnowledgeAcquirer::default();
    }
}
