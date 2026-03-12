//! Cognitive dispatch helpers — analysis, classification, reporting, and sister tool calls.
//!
//! Extracted from `cognitive.rs` for compilation performance.
//! Contains: perceive_beliefs, memory ops, detection heuristics,
//! complexity/risk classification, status reporting, and capabilities prompt.

use super::super::connection::{extract_text, SisterConnection};
use super::Sisters;

impl Sisters {
    /// Query beliefs from the Cognition sister (user model beliefs)
    pub async fn perceive_beliefs(&self, text: &str) -> Option<String> {
        if let Some(s) = &self.cognition {
            let result = s.call_tool("cognition_belief_query", serde_json::json!({
                "context": text,
                "limit": 10,
            })).await.ok();
            result.map(|v| extract_text(&v)).filter(|t| !t.is_empty())
        } else { None }
    }

    /// Save a fact/conversation to memory
    pub async fn _save_to_memory(&self, content: &str, event_type: &str) {
        if let Some(mem) = &self.memory {
            let _ = mem
                .call_tool(
                    "memory_add",
                    serde_json::json!({
                        "content": content,
                        "event_type": event_type,
                    }),
                )
                .await;
        }
    }

    /// Log conversation exchange to memory
    pub async fn log_conversation(&self, user_msg: &str, assistant_msg: &str) {
        if let Some(mem) = &self.memory {
            let _ = mem
                .call_tool(
                    "conversation_log",
                    serde_json::json!({
                        "role": "user",
                        "content": user_msg,
                    }),
                )
                .await;
            let _ = mem
                .call_tool(
                    "conversation_log",
                    serde_json::json!({
                        "role": "assistant",
                        "content": assistant_msg,
                    }),
                )
                .await;
        }
    }

    /// Graceful degradation: report what's available and what's offline
    pub fn degradation_report(&self) -> String {
        let mut online = Vec::new();
        let mut offline = Vec::new();
        for (name, opt) in self.all_sisters() {
            if opt.is_some() {
                online.push(name);
            } else {
                offline.push(name);
            }
        }
        let total = online.len() + offline.len();
        if offline.is_empty() {
            format!("All {} sisters online: {}", total, online.join(", "))
        } else {
            format!(
                "{}/{} sisters online: {}. Offline: {}. Capabilities degraded for: {}",
                online.len(),
                total,
                online.join(", "),
                offline.join(", "),
                offline.iter().map(|n| match *n {
                    "Memory" => "persistent memory",
                    "Identity" => "receipt signing",
                    "Codebase" => "code analysis",
                    "Vision" => "visual capture/web browsing",
                    "Comm" => "inter-agent messaging",
                    "Contract" => "policy enforcement",
                    "Time" => "temporal reasoning",
                    "Planning" => "goal tracking",
                    "Cognition" => "user modeling",
                    "Reality" => "environment detection",
                    "Forge" => "blueprint generation",
                    "Aegis" => "safety validation",
                    "Veritas" => "intent compilation",
                    "Evolve" => "skill crystallization",
                    _ => "unknown",
                }).collect::<Vec<_>>().join(", ")
            )
        }
    }

    /// Detect if input involves code operations
    pub fn detects_code(text: &str) -> bool {
        let lower = text.to_lowercase();
        // Only match when the context is clearly about code/programming,
        // not casual mentions like "my favorite database" or "fix my schedule".
        let code_keywords = [
            "function ", "class ", "module ", "compile", "refactor",
            "api endpoint", "implement ", "debug ", "import ",
            "dependency", "crate ", "package.json",
            "frontend", "backend", "web app", "webapp",
            ".rs", ".ts", ".py", ".js", ".go", ".java", "src/", "crates/",
        ];
        code_keywords.iter().any(|kw| lower.contains(kw))
    }

    /// Detect if input involves visual content
    pub fn detects_visual(text: &str) -> bool {
        let lower = text.to_lowercase();
        let keywords = [
            "screenshot", "image", "photo", "picture", "screen", "ui",
            "layout", "design", "visual", "display", "render",
            ".png", ".jpg", ".svg", ".gif",
        ];
        keywords.iter().any(|kw| lower.contains(kw))
    }

    /// Classify intent complexity: simple queries vs complex tasks
    pub fn classify_complexity(text: &str) -> &'static str {
        let lower = text.trim().to_lowercase();
        let word_count = lower.split_whitespace().count();

        if word_count <= 3 {
            let greetings = ["hi", "hey", "hello", "yo", "sup", "howdy", "morning",
                "afternoon", "evening", "thanks", "thank you", "bye", "goodbye"];
            if greetings.iter().any(|g| lower.contains(g)) {
                return "simple";
            }
        }

        let complex_keywords = [
            "build", "create", "implement", "develop", "design", "write",
            "generate", "deploy", "setup", "set up", "configure", "install",
            "migrate", "refactor", "analyze", "scaffold", "architect",
            "ecommerce", "e-commerce", "website", "application", "project",
            "run it", "start it", "launch it", "do it", "go ahead",
        ];
        if complex_keywords.iter().any(|kw| lower.contains(kw)) {
            return "complex";
        }

        if Self::detects_code(&lower) {
            return "moderate";
        }

        if word_count <= 8 {
            "simple"
        } else {
            "moderate"
        }
    }

    /// Assess risk level of an action
    pub fn assess_risk(text: &str) -> &'static str {
        let lower = text.to_lowercase();

        let high_risk = ["delete", "remove", "drop", "rm -rf", "format", "wipe",
            "send email", "send message", "execute", "sudo", "chmod"];
        if high_risk.iter().any(|kw| lower.contains(kw)) {
            return "high";
        }

        let medium_risk = ["modify", "update", "change", "write", "overwrite",
            "install", "uninstall", "deploy", "push"];
        if medium_risk.iter().any(|kw| lower.contains(kw)) {
            return "medium";
        }

        if Self::detects_code(&lower) {
            return "low";
        }

        "none"
    }

    /// Get a status summary of connected sisters
    pub fn status_summary(&self) -> String {
        let mut parts = Vec::new();
        for (name, opt) in self.all_sisters() {
            if let Some(conn) = opt {
                parts.push(format!("{} ({} tools)", name, conn.tools.len()));
            }
        }
        if parts.is_empty() {
            "No sisters connected".to_string()
        } else {
            parts.join(", ")
        }
    }

    /// Build a system prompt section describing available sister capabilities
    pub fn capabilities_prompt(&self) -> String {
        let descriptions: Vec<(&str, &Option<SisterConnection>, &str)> = vec![
            ("Memory", &self.memory, "Persistent memory, recall, conversation history"),
            ("Identity", &self.identity, "User identity, action receipts, cryptographic signing"),
            ("Codebase", &self.codebase, "Code analysis, search, impact assessment, file operations"),
            ("Vision", &self.vision, "Image processing, screen capture, visual understanding"),
            ("Comm", &self.comm, "Communication, messaging, notifications"),
            ("Contract", &self.contract, "Policy checking, behavioral contracts, safety rules"),
            ("Time", &self.time, "Temporal context, scheduling, duration tracking, deadlines"),
            ("Planning", &self.planning, "Goal decomposition, task planning, step-by-step execution"),
            ("Cognition", &self.cognition, "User model, belief revision, decision patterns, preferences"),
            ("Reality", &self.reality, "Environment awareness, deployment context, system state"),
            ("Forge", &self.forge, "Code generation, architecture blueprints, project scaffolding"),
            ("Aegis", &self.aegis, "Safety validation, shadow simulation, harm prevention"),
            ("Veritas", &self.veritas, "Intent verification, uncertainty detection, truth assessment"),
            ("Evolve", &self.evolve, "Skill crystallization, pattern learning, capability growth"),
        ];

        let mut sections = Vec::new();
        for (name, opt, desc) in &descriptions {
            if opt.is_some() {
                sections.push(format!("- **{}**: {}", name, desc));
            }
        }

        if sections.is_empty() {
            String::new()
        } else {
            format!(
                "\n\n# Connected Sisters ({}/{})\n{}",
                self.connected_count(),
                self.all_sisters().len(),
                sections.join("\n")
            )
        }
    }
}
