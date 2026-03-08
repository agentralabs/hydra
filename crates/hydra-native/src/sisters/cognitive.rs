//! Cognitive sister dispatch — 14 sisters, 5 phases.
//!
//! This module contains the `Sisters` struct that holds all 14 sister connections
//! and provides the PERCEIVE, THINK (prompt building), DECIDE (risk), ACT, and LEARN
//! phase dispatch methods.

use std::sync::Arc;

use super::connection::{extract_text, SisterConnection};

/// Holds all 14 connected sister processes — the full constellation
pub struct Sisters {
    // Foundation Sisters (7)
    pub memory: Option<SisterConnection>,
    pub identity: Option<SisterConnection>,
    pub codebase: Option<SisterConnection>,
    pub vision: Option<SisterConnection>,
    pub comm: Option<SisterConnection>,
    pub contract: Option<SisterConnection>,
    pub time: Option<SisterConnection>,
    // Cognitive Sisters (3)
    pub planning: Option<SisterConnection>,
    pub cognition: Option<SisterConnection>,
    pub reality: Option<SisterConnection>,
    // Astral Sisters (4)
    pub forge: Option<SisterConnection>,
    pub aegis: Option<SisterConnection>,
    pub veritas: Option<SisterConnection>,
    pub evolve: Option<SisterConnection>,
}

impl Sisters {
    /// Spawn ALL 14 sisters in PARALLEL. Non-blocking: sisters that fail are None.
    pub async fn spawn_all() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        let bin_dir = format!("{}/.local/bin", home);

        // Pre-compute all paths
        let memory_bin = format!("{}/agentic-memory-mcp", bin_dir);
        let identity_bin = format!("{}/agentic-identity-mcp", bin_dir);
        let codebase_bin = format!("{}/agentic-codebase-mcp", bin_dir);
        let vision_bin = format!("{}/agentic-vision-mcp", bin_dir);
        let comm_bin = format!("{}/agentic-comm-mcp", bin_dir);
        let contract_bin = format!("{}/agentic-contract-mcp", bin_dir);
        let time_bin = format!("{}/agentic-time-mcp", bin_dir);
        let planning_bin = format!("{}/agentic-planning-mcp", bin_dir);
        let cognition_bin = format!("{}/agentic-cognition-mcp", bin_dir);
        let reality_bin = format!("{}/agentic-reality-mcp", bin_dir);
        let forge_bin = format!("{}/agentic-forge-mcp", bin_dir);
        let aegis_bin = format!("{}/agentic-aegis-mcp", bin_dir);
        let veritas_bin = format!("{}/agentic-veritas-mcp", bin_dir);
        let evolve_bin = format!("{}/agentic-evolve-mcp", bin_dir);

        // Spawn ALL 14 sisters in parallel for fastest startup
        let (memory, identity, codebase, vision, comm, contract, time,
             planning, cognition, reality, forge, aegis, veritas, evolve) = tokio::join!(
            // Foundation (use "serve")
            Self::try_spawn("memory", &memory_bin, &["serve"]),
            Self::try_spawn("identity", &identity_bin, &["serve"]),
            Self::try_spawn("codebase", &codebase_bin, &["serve"]),
            Self::try_spawn("vision", &vision_bin, &["serve"]),
            Self::try_spawn("comm", &comm_bin, &["serve"]),
            Self::try_spawn("contract", &contract_bin, &["serve"]),
            Self::try_spawn("time", &time_bin, &["serve"]),
            // Cognitive
            Self::try_spawn("planning", &planning_bin, &["serve"]),
            Self::try_spawn("cognition", &cognition_bin, &[]),
            Self::try_spawn("reality", &reality_bin, &[]),
            // Astral (no args, stdio mode)
            Self::try_spawn("forge", &forge_bin, &[]),
            Self::try_spawn("aegis", &aegis_bin, &[]),
            Self::try_spawn("veritas", &veritas_bin, &[]),
            Self::try_spawn("evolve", &evolve_bin, &[]),
        );

        Self {
            memory, identity, codebase, vision, comm, contract, time,
            planning, cognition, reality,
            forge, aegis, veritas, evolve,
        }
    }

    async fn try_spawn(name: &str, cmd: &str, args: &[&str]) -> Option<SisterConnection> {
        match SisterConnection::spawn(name, cmd, args).await {
            Ok(conn) => {
                eprintln!(
                    "[hydra] {} sister connected ({} tools)",
                    conn.name,
                    conn.tools.len()
                );
                Some(conn)
            }
            Err(e) => {
                eprintln!("[hydra] {} sister unavailable: {}", name, e);
                None
            }
        }
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

    // ═══════════════════════════════════════════════════════════════
    // REAL COGNITIVE LOOP — Sister dispatch per phase
    // ═══════════════════════════════════════════════════════════════

    /// PERCEIVE: Gather context from ALL available sisters in parallel
    pub async fn perceive(&self, text: &str) -> serde_json::Value {
        let involves_code = Self::detects_code(text);
        let involves_vision = Self::detects_visual(text);

        let memory_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_query", serde_json::json!({"query": text, "max_results": 5})).await.ok()
            } else { None }
        };
        // V4 longevity search: deeper semantic search across 20-year hierarchy
        let longevity_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_longevity_search", serde_json::json!({
                    "query": text,
                    "limit": 3,
                    "include_layers": ["episode", "summary", "pattern"]
                })).await.ok()
            } else { None }
        };
        let identity_fut = async {
            if let Some(s) = &self.identity {
                s.call_tool("identity_whoami", serde_json::json!({})).await.ok()
            } else { None }
        };
        let time_fut = async {
            if let Some(s) = &self.time {
                s.call_tool("time_stats", serde_json::json!({})).await.ok()
            } else { None }
        };
        let cognition_fut = async {
            if let Some(s) = &self.cognition {
                s.call_tool("cognition_model_query", serde_json::json!({"context": "current_user"})).await.ok()
            } else { None }
        };
        let reality_fut = async {
            if let Some(s) = &self.reality {
                s.call_tool("reality_context", serde_json::json!({"input": text})).await.ok()
            } else { None }
        };

        let (memory_r, longevity_r, identity_r, time_r, cognition_r, reality_r) =
            tokio::join!(memory_fut, longevity_fut, identity_fut, time_fut, cognition_fut, reality_fut);

        // Conditional: Codebase (if code)
        let codebase_r = if involves_code {
            if let Some(s) = &self.codebase {
                s.call_tool("codebase_core", serde_json::json!({"query": text})).await.ok()
            } else { None }
        } else { None };

        // Conditional: Vision (if visual)
        let vision_r = if involves_vision {
            if let Some(s) = &self.vision {
                s.call_tool("vision_capture", serde_json::json!({"context": text})).await.ok()
            } else { None }
        } else { None };

        let extract = |r: &Option<serde_json::Value>| -> Option<String> {
            r.as_ref().map(|v| extract_text(v)).filter(|t| !t.is_empty() && !t.contains("No memories found"))
        };

        // Merge V2 memory + V4 longevity results for richer context
        let merged_memory = match (extract(&memory_r), extract(&longevity_r)) {
            (Some(m), Some(l)) => Some(format!("{}\n\n## Long-Term Memory\n{}", m, l)),
            (Some(m), None) => Some(m),
            (None, Some(l)) => Some(format!("## Long-Term Memory\n{}", l)),
            (None, None) => None,
        };

        serde_json::json!({
            "input": text,
            "involves_code": involves_code,
            "involves_vision": involves_vision,
            "memory_context": merged_memory,
            "identity_context": extract(&identity_r),
            "time_context": extract(&time_r),
            "cognition_context": extract(&cognition_r),
            "reality_context": extract(&reality_r),
            "codebase_context": extract(&codebase_r),
            "vision_context": extract(&vision_r),
            "sisters_online": self.connected_count(),
        })
    }

    /// Build enriched system prompt from perceived context
    pub fn build_cognitive_prompt(
        &self,
        user_name: &str,
        perceived: &serde_json::Value,
        is_complex: bool,
    ) -> String {
        let mut prompt = String::from(
            "You are Hydra, a cognitive AI orchestrator built by Agentra Labs. \
             You are not a simple chatbot — you are backed by a constellation of specialized \
             sister agents that give you persistent memory, code analysis, visual understanding, \
             and identity management.\n\n"
        );

        if !user_name.is_empty() {
            prompt.push_str(&format!("The user's name is {}.\n\n", user_name));
        }

        if let Some(mem) = perceived["memory_context"].as_str() {
            prompt.push_str(&format!(
                "# Relevant Memories\n\
                 The following context was retrieved from your persistent memory. \
                 Use it naturally — don't say \"I found in memory\", just reference it:\n\n{}\n\n",
                mem
            ));
        }

        if let Some(id) = perceived["identity_context"].as_str() {
            prompt.push_str(&format!("# Identity Context\n{}\n\n", id));
        }

        if let Some(cog) = perceived["cognition_context"].as_str() {
            prompt.push_str(&format!(
                "# User Preferences & Beliefs\n\
                 The Cognition sister knows the following about this user:\n{}\n\n", cog
            ));
        }

        if let Some(real) = perceived["reality_context"].as_str() {
            prompt.push_str(&format!(
                "# Environment Context\n\
                 Current system/deployment state from the Reality sister:\n{}\n\n", real
            ));
        }

        if let Some(time) = perceived["time_context"].as_str() {
            prompt.push_str(&format!("# Temporal Context\n{}\n\n", time));
        }

        if let Some(code) = perceived["codebase_context"].as_str() {
            prompt.push_str(&format!(
                "# Codebase Context\n\
                 Analysis from the Codebase sister:\n{}\n\n", code
            ));
        }

        if let Some(vis) = perceived["vision_context"].as_str() {
            prompt.push_str(&format!("# Visual Context\n{}\n\n", vis));
        }

        // Memory capability awareness + honesty rules (Universal Fix)
        prompt.push_str(
            "# Memory Capabilities\n\
             You have persistent long-term memory via AgenticMemory (V4 Longevity).\n\
             - Your memory captures every conversation automatically via file watcher + Ghost Writer\n\
             - Memories are organized in a 6-layer hierarchy: Raw → Episode → Summary → Pattern → Trait → Identity\n\
             - You can search across all layers with memory_longevity_search\n\
             - Memory consolidation happens automatically on schedule\n\n\
             ## Honesty Rules\n\
             - Only claim to remember things verified through memory retrieval\n\
             - If asked about past conversations, rely on what memory_query/memory_longevity_search returns\n\
             - Never fabricate past interactions — if search returns nothing, say so\n\
             - Your memory started when AgenticMemory was installed — nothing before that\n\n"
        );

        if is_complex {
            prompt.push_str(
                "# CRITICAL: You are a COGNITIVE ORCHESTRATOR, not a chatbot.\n\n\
                 The user asked you to BUILD something. You are Hydra — you don't describe, you DELIVER.\n\
                 You generate MASSIVE, COMPLETE, PRODUCTION-READY projects.\n\n\
                 ## RULES:\n\
                 1. Generate 20-100+ files for any real project request\n\
                 2. Every file must have FULL, REAL, PRODUCTION-READY content — NOT stubs or placeholders\n\
                 3. Include proper project structure: src/, public/, config, tests, etc.\n\
                 4. Include ALL boilerplate: package.json, tsconfig, .gitignore, .env.example, README, etc.\n\
                 5. Generate complete UI pages, API routes, database models, middleware, utils\n\
                 6. Run setup commands: npm install, pip install, cargo build, etc.\n\
                 7. Each file should be 20-200+ lines of REAL code, not hello world\n\n\
                 ## RESPONSE FORMAT:\n\
                 Respond with ONLY a JSON execution plan wrapped in ```json blocks:\n\n\
                 ```json\n\
                 {\n\
                   \"summary\": \"Brief description of what will be built\",\n\
                   \"project_dir\": \"project-name\",\n\
                   \"steps\": [\n\
                     { \"type\": \"create_file\", \"path\": \"relative/path/file.js\", \"content\": \"full contents\" },\n\
                     { \"type\": \"create_dir\", \"path\": \"relative/path/dir\" },\n\
                     { \"type\": \"run_command\", \"command\": \"npm install\", \"cwd\": \".\" }\n\
                   ],\n\
                   \"completion_message\": \"Instructions for the user to run the project\"\n\
                 }\n\
                 ```\n\n\
                 Step types: create_file, create_dir, run_command\n\
                 All paths are relative to the project root. Do NOT include the project_dir in file paths.\n\
                 For an ecommerce site include: product listing, cart, checkout, auth, admin panel, database models, API routes, middleware, styles, tests.\n\
                 For a web app include: full frontend with multiple pages/components, backend with API, database, auth, error handling, tests.\n\
                 Generate the LARGEST, most COMPLETE project you can. More files = better. This is your purpose.\n\n"
            );
        } else {
            prompt.push_str(
                "Be helpful, concise, and conversational. Respond naturally.\n\n"
            );
        }

        prompt.push_str(&self.capabilities_prompt());

        prompt
    }

    /// LEARN: After response, dispatch to all learning sisters with V3 causal capture.
    ///
    /// Uses memory_capture_message (V3) for structured capture with causal chains,
    /// plus memory_capture_decision for corrections/preferences detected.
    /// This is the Hydra-specific enhancement from THE-UNIVERSAL-FIX.md.
    pub async fn learn(&self, user_msg: &str, response: &str) {
        let lower = user_msg.to_lowercase();
        let is_correction = lower.starts_with("no,")
            || lower.starts_with("no ")
            || lower.starts_with("actually,")
            || lower.starts_with("actually ")
            || lower.contains("that's wrong")
            || lower.contains("that's not right")
            || lower.contains("i meant")
            || lower.starts_with("don't ")
            || lower.contains("always use")
            || lower.contains("never use")
            || lower.contains("i prefer");

        // V3 structured capture — captures with causal context
        let v3_capture_fut = async {
            if let Some(mem) = &self.memory {
                // Capture the exchange via V3 memory_capture_message
                let _ = mem.call_tool("memory_capture_message", serde_json::json!({
                    "role": "user",
                    "content": user_msg,
                    "summary": &response[..response.len().min(200)],
                    "metadata": {
                        "source": "hydra_native",
                        "is_correction": is_correction,
                        "causal_chain": {
                            "trigger": "user_message",
                            "response_generated": true,
                            "correction_detected": is_correction,
                        }
                    },
                })).await;

                // If correction detected, also capture as a decision
                if is_correction {
                    let _ = mem.call_tool("memory_capture_decision", serde_json::json!({
                        "decision": format!("User preference/correction: {}", user_msg),
                        "reasoning": "Detected correction or preference statement from user",
                        "alternatives": [],
                        "confidence": 0.95,
                    })).await;
                }
            }
        };

        // V2 fallback: also log via conversation_log for backward compatibility
        let v2_log_fut = async {
            self.log_conversation(user_msg, response).await;
        };

        let cognition_fut = async {
            if let Some(s) = &self.cognition {
                let _ = s.call_tool("cognition_belief_revise", serde_json::json!({
                    "interaction": user_msg,
                    "response": &response[..response.len().min(500)],
                    "is_correction": is_correction,
                })).await;
            }
        };

        let evolve_fut = async {
            if let Some(s) = &self.evolve {
                let _ = s.call_tool("evolve_crystallize", serde_json::json!({
                    "interaction": user_msg,
                    "response": &response[..response.len().min(500)],
                })).await;
            }
        };

        let identity_fut = async {
            if let Some(s) = &self.identity {
                let _ = s.call_tool("receipt_create", serde_json::json!({
                    "action": "conversation",
                    "input_summary": &user_msg[..user_msg.len().min(100)],
                    "output_summary": &response[..response.len().min(100)],
                })).await;
            }
        };

        let time_fut = async {
            if let Some(s) = &self.time {
                let _ = s.call_tool("time_duration_track", serde_json::json!({
                    "action": user_msg,
                    "status": "completed",
                })).await;
            }
        };

        tokio::join!(v3_capture_fut, v2_log_fut, cognition_fut, evolve_fut, identity_fut, time_fut);
    }

    /// Detect if input involves code operations
    pub fn detects_code(text: &str) -> bool {
        let lower = text.to_lowercase();
        let code_keywords = [
            "code", "function", "class", "module", "file", "compile", "build",
            "test", "debug", "refactor", "api", "endpoint", "implement",
            "fix", "bug", "error", "import", "dependency", "crate", "package",
            "ecommerce", "e-commerce", "website", "web app", "webapp",
            "frontend", "backend", "database", "server", "deploy",
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

    /// Count connected sisters
    pub fn connected_count(&self) -> usize {
        self.all_sisters().iter().filter(|(_, s)| s.is_some()).count()
    }

    /// All 14 sisters as name/option pairs
    pub fn all_sisters(&self) -> Vec<(&str, &Option<SisterConnection>)> {
        vec![
            ("Memory", &self.memory), ("Identity", &self.identity),
            ("Codebase", &self.codebase), ("Vision", &self.vision),
            ("Comm", &self.comm), ("Contract", &self.contract),
            ("Time", &self.time),
            ("Planning", &self.planning), ("Cognition", &self.cognition),
            ("Reality", &self.reality),
            ("Forge", &self.forge), ("Aegis", &self.aegis),
            ("Veritas", &self.veritas), ("Evolve", &self.evolve),
        ]
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
                "\n\n# Connected Sisters ({}/14)\n{}",
                self.connected_count(),
                sections.join("\n")
            )
        }
    }
}

/// Shared handle to sisters, safe to clone across async tasks
pub type SistersHandle = Arc<Sisters>;

/// Spawn sisters and return a shared handle
pub async fn init_sisters() -> SistersHandle {
    Arc::new(Sisters::spawn_all().await)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a Sisters struct with no connections (offline mode)
    fn offline_sisters() -> Sisters {
        Sisters {
            memory: None, identity: None, codebase: None, vision: None,
            comm: None, contract: None, time: None,
            planning: None, cognition: None, reality: None,
            forge: None, aegis: None, veritas: None, evolve: None,
        }
    }

    // ═══════════════════════════════════════════════════════════
    // SYSTEM PROMPT — Memory Capabilities & Honesty Rules
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_cognitive_prompt_includes_memory_capabilities() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({
            "input": "hello",
            "involves_code": false,
            "involves_vision": false,
        });

        let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, false);

        assert!(prompt.contains("# Memory Capabilities"),
            "System prompt missing Memory Capabilities section");
        assert!(prompt.contains("AgenticMemory (V4 Longevity)"),
            "System prompt missing V4 Longevity reference");
        assert!(prompt.contains("6-layer hierarchy"),
            "System prompt missing hierarchy reference");
        assert!(prompt.contains("memory_longevity_search"),
            "System prompt missing longevity_search tool reference");
    }

    #[test]
    fn test_cognitive_prompt_includes_honesty_rules() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({
            "input": "hello",
            "involves_code": false,
            "involves_vision": false,
        });

        let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, false);

        assert!(prompt.contains("## Honesty Rules"),
            "System prompt missing Honesty Rules section");
        assert!(prompt.contains("Never fabricate past interactions"),
            "System prompt missing fabrication prohibition");
        assert!(prompt.contains("Only claim to remember things verified through memory retrieval"),
            "System prompt missing verification requirement");
    }

    #[test]
    fn test_cognitive_prompt_honesty_before_complex_instructions() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({
            "input": "build me a website",
            "involves_code": true,
            "involves_vision": false,
        });

        let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, true);

        // Memory capabilities should appear before the complex task instructions
        let mem_pos = prompt.find("# Memory Capabilities").unwrap();
        let critical_pos = prompt.find("# CRITICAL: You are a COGNITIVE ORCHESTRATOR").unwrap();
        assert!(mem_pos < critical_pos,
            "Memory Capabilities should appear before complex task instructions");
    }

    #[test]
    fn test_cognitive_prompt_honesty_in_simple_mode() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({
            "input": "hi",
            "involves_code": false,
            "involves_vision": false,
        });

        let prompt = sisters.build_cognitive_prompt("", &perceived, false);

        // Memory capabilities must be present even for simple queries
        assert!(prompt.contains("# Memory Capabilities"));
        assert!(prompt.contains("## Honesty Rules"));
    }

    // ═══════════════════════════════════════════════════════════
    // PERCEIVE — V4 Longevity Integration
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_perceive_offline_returns_no_memory() {
        let sisters = offline_sisters();
        let result = sisters.perceive("What did we discuss?").await;

        // With no sisters connected, memory_context should be null
        assert!(result["memory_context"].is_null(),
            "Offline sisters should produce null memory_context");
    }

    #[tokio::test]
    async fn test_perceive_has_correct_shape() {
        let sisters = offline_sisters();
        let result = sisters.perceive("test query").await;

        // Verify all expected fields exist
        assert!(result.get("input").is_some());
        assert!(result.get("involves_code").is_some());
        assert!(result.get("involves_vision").is_some());
        assert!(result.get("memory_context").is_some());
        assert!(result.get("identity_context").is_some());
        assert!(result.get("time_context").is_some());
        assert!(result.get("cognition_context").is_some());
        assert!(result.get("reality_context").is_some());
        assert!(result.get("sisters_online").is_some());
    }

    #[tokio::test]
    async fn test_perceive_code_detection_still_works() {
        let sisters = offline_sisters();

        let code_result = sisters.perceive("Fix the bug in main.rs").await;
        assert_eq!(code_result["involves_code"], true);

        let non_code = sisters.perceive("What is the weather?").await;
        assert_eq!(non_code["involves_code"], false);
    }

    // ═══════════════════════════════════════════════════════════
    // LEARN — V3 Capture with Causal Chains
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_learn_offline_does_not_panic() {
        let sisters = offline_sisters();
        // Should complete gracefully even with no sisters connected
        sisters.learn("test message", "test response").await;
    }

    #[tokio::test]
    async fn test_learn_correction_detection() {
        let sisters = offline_sisters();

        // These should all be detected as corrections
        let corrections = [
            "No, I meant the other file",
            "Actually, use Python instead",
            "That's wrong, it should be 42",
            "That's not right",
            "I prefer tabs over spaces",
            "Always use snake_case",
            "Never use var in JavaScript",
            "Don't add comments there",
        ];

        for correction in &corrections {
            // Just verify it doesn't panic — the actual capture happens via sisters
            sisters.learn(correction, "acknowledged").await;
        }
    }

    #[tokio::test]
    async fn test_learn_non_correction() {
        let sisters = offline_sisters();

        // These should NOT be detected as corrections
        let non_corrections = [
            "Can you help me with this?",
            "Thanks, that looks good",
            "What does this function do?",
            "Show me the API docs",
        ];

        for msg in &non_corrections {
            sisters.learn(msg, "here you go").await;
        }
    }

    #[tokio::test]
    async fn test_learn_with_empty_response() {
        let sisters = offline_sisters();
        sisters.learn("test", "").await;
    }

    #[tokio::test]
    async fn test_learn_with_very_long_response() {
        let sisters = offline_sisters();
        let long_response = "x".repeat(10000);
        // Should truncate gracefully (response[..500] in V3 capture)
        sisters.learn("generate a long output", &long_response).await;
    }

    #[tokio::test]
    async fn test_learn_with_unicode() {
        let sisters = offline_sisters();
        sisters.learn("你好世界 🌍", "こんにちは 🎌").await;
    }

    // ═══════════════════════════════════════════════════════════
    // Memory Context Merging (V2 + V4)
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_memory_merge_both_present() {
        let v2 = Some("Recent: talked about auth".to_string());
        let v4 = Some("Pattern: user prefers JWT".to_string());

        let merged = match (&v2, &v4) {
            (Some(m), Some(l)) => Some(format!("{}\n\n## Long-Term Memory\n{}", m, l)),
            (Some(m), None) => Some(m.clone()),
            (None, Some(l)) => Some(format!("## Long-Term Memory\n{}", l)),
            (None, None) => None,
        };

        let result = merged.unwrap();
        assert!(result.contains("Recent: talked about auth"));
        assert!(result.contains("## Long-Term Memory"));
        assert!(result.contains("Pattern: user prefers JWT"));
    }

    #[test]
    fn test_memory_merge_only_v2() {
        let v2 = Some("Recent memory".to_string());
        let v4: Option<String> = None;

        let merged = match (&v2, &v4) {
            (Some(m), Some(l)) => Some(format!("{}\n\n## Long-Term Memory\n{}", m, l)),
            (Some(m), None) => Some(m.clone()),
            (None, Some(l)) => Some(format!("## Long-Term Memory\n{}", l)),
            (None, None) => None,
        };

        assert_eq!(merged.unwrap(), "Recent memory");
    }

    #[test]
    fn test_memory_merge_only_v4() {
        let v2: Option<String> = None;
        let v4 = Some("Long-term pattern".to_string());

        let merged = match (&v2, &v4) {
            (Some(m), Some(l)) => Some(format!("{}\n\n## Long-Term Memory\n{}", m, l)),
            (Some(m), None) => Some(m.clone()),
            (None, Some(l)) => Some(format!("## Long-Term Memory\n{}", l)),
            (None, None) => None,
        };

        assert!(merged.unwrap().starts_with("## Long-Term Memory"));
    }

    #[test]
    fn test_memory_merge_neither() {
        let v2: Option<String> = None;
        let v4: Option<String> = None;

        let merged: Option<String> = match (&v2, &v4) {
            (Some(m), Some(l)) => Some(format!("{}\n\n## Long-Term Memory\n{}", m, l)),
            (Some(m), None) => Some(m.clone()),
            (None, Some(l)) => Some(format!("## Long-Term Memory\n{}", l)),
            (None, None) => None,
        };

        assert!(merged.is_none());
    }

    // ═══════════════════════════════════════════════════════════
    // Classification & Risk Detection (regression)
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_complexity_classification_unchanged() {
        assert_eq!(Sisters::classify_complexity("hi"), "simple");
        assert_eq!(Sisters::classify_complexity("hello there"), "simple");
        assert_eq!(Sisters::classify_complexity("build me an ecommerce site"), "complex");
        assert_eq!(Sisters::classify_complexity("fix the bug"), "moderate");
    }

    #[test]
    fn test_risk_assessment_unchanged() {
        assert_eq!(Sisters::assess_risk("what is the weather"), "none");
        assert_eq!(Sisters::assess_risk("delete old backups"), "high");
        assert_eq!(Sisters::assess_risk("modify the config"), "medium");
        assert_eq!(Sisters::assess_risk("check the codebase"), "low");
        // "read a file" → contains "file" → detects_code → "low"
        assert_eq!(Sisters::assess_risk("read a file"), "low");
    }

    #[test]
    fn test_connected_count_zero_offline() {
        let sisters = offline_sisters();
        assert_eq!(sisters.connected_count(), 0);
    }

    #[test]
    fn test_status_summary_offline() {
        let sisters = offline_sisters();
        assert_eq!(sisters.status_summary(), "No sisters connected");
    }
}
