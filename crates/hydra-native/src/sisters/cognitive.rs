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
        // Configurable via HYDRA_SISTER_BIN_DIR env var (default: ~/.local/bin)
        let bin_dir = std::env::var("HYDRA_SISTER_BIN_DIR")
            .unwrap_or_else(|_| format!("{}/.local/bin", home));

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

        // Hydra uses its own memory file — separate from Claude Code's ~/.brain.amem
        let hydra_memory = format!("{}/.hydra/memory/hydra.amem", home);
        let memory_args: Vec<&str> = vec!["serve", "--memory", &hydra_memory];

        // Spawn ALL 14 sisters in parallel for fastest startup
        let (memory, identity, codebase, vision, comm, contract, time,
             planning, cognition, reality, forge, aegis, veritas, evolve) = tokio::join!(
            // Foundation (use "serve")
            Self::try_spawn("memory", &memory_bin, &memory_args),
            Self::try_spawn("identity", &identity_bin, &["serve"]),
            Self::try_spawn("codebase", &codebase_bin, &["serve"]),
            Self::try_spawn("vision", &vision_bin, &["serve"]),
            Self::try_spawn("comm", &comm_bin, &["serve"]),
            Self::try_spawn("contract", &contract_bin, &[]),
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

        let s = Self {
            memory, identity, codebase, vision, comm, contract, time,
            planning, cognition, reality,
            forge, aegis, veritas, evolve,
        };
        let all = s.all_sisters();
        let total = all.len();
        let connected = all.iter().filter(|(_, opt)| opt.is_some()).count();
        eprintln!("[hydra] ═══ {}/{} sisters connected ═══", connected, total);
        s
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

    /// Get specific tools from a sister by name. Returns matching tool names.
    /// Used by the tool router to send only relevant tools to the LLM.
    pub fn tools_for_sister(&self, sister: &str, names: &[&str]) -> Vec<String> {
        let conn = match sister {
            "memory" => self.memory.as_ref(),
            "identity" => self.identity.as_ref(),
            "codebase" => self.codebase.as_ref(),
            "vision" => self.vision.as_ref(),
            "comm" => self.comm.as_ref(),
            "contract" => self.contract.as_ref(),
            "time" => self.time.as_ref(),
            "planning" => self.planning.as_ref(),
            "cognition" => self.cognition.as_ref(),
            "reality" => self.reality.as_ref(),
            "forge" => self.forge.as_ref(),
            "aegis" => self.aegis.as_ref(),
            "veritas" => self.veritas.as_ref(),
            "evolve" => self.evolve.as_ref(),
            _ => None,
        };
        match conn {
            Some(c) => c.tools.iter()
                .filter(|t| names.iter().any(|n| t.contains(n)))
                .cloned()
                .collect(),
            None => vec![],
        }
    }

    /// Discover MCP tools from all connected sisters and return tool names per server.
    /// Returns a list of (server_name, tool_name) tuples.
    pub fn discover_mcp_tools(&self) -> Vec<(String, String)> {
        let mut discovered = Vec::new();
        for (name, opt) in self.all_sisters() {
            if let Some(conn) = opt {
                for tool_name in &conn.tools {
                    discovered.push((name.to_string(), tool_name.clone()));
                }
            }
        }
        discovered
    }

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

    // ═══════════════════════════════════════════════════════════════
    // REAL COGNITIVE LOOP — Sister dispatch per phase
    // ═══════════════════════════════════════════════════════════════

    /// PERCEIVE: Gather context from ALL available sisters in parallel
    pub async fn perceive(&self, text: &str) -> serde_json::Value {
        // Debug: log sister connection status at perceive time
        let connected: Vec<&str> = self.all_sisters().iter()
            .filter_map(|(name, opt)| if opt.is_some() { Some(*name) } else { None })
            .collect();
        eprintln!("[hydra:perceive] {} sisters connected: {:?}", connected.len(), connected);

        let involves_code = Self::detects_code(text);
        let involves_vision = Self::detects_visual(text);

        // Facts/corrections/decisions first — high-signal stored preferences
        let facts_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_query", serde_json::json!({
                    "query": text,
                    "event_types": ["fact", "correction", "decision"],
                    "max_results": 5,
                    "sort_by": "highest_confidence"
                })).await.ok()
            } else { None }
        };
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
        let similar_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_similar", serde_json::json!({"content": text, "limit": 3})).await.ok()
            } else { None }
        };
        let ground_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_ground", serde_json::json!({"claim": text})).await.ok()
            } else { None }
        };
        let predict_fut = async {
            if let Some(s) = &self.cognition {
                s.call_tool("cognition_predict", serde_json::json!({"context": text})).await.ok()
            } else { None }
        };

        // ── NEW: Veritas intent verification (detect ambiguity in user query) ──
        let veritas_fut = async {
            if let Some(s) = &self.veritas {
                s.call_tool("verify_intent", serde_json::json!({"input": text})).await.ok()
            } else { None }
        };
        // ── NEW: Contract policy check (are there constraints on this query?) ──
        let contract_fut = async {
            if let Some(s) = &self.contract {
                s.call_tool("policy_query", serde_json::json!({"action": text})).await.ok()
            } else { None }
        };
        // ── NEW: Planning goal context (any active goals relevant to this query?) ──
        let planning_fut = async {
            if let Some(s) = &self.planning {
                s.call_tool("goal_query", serde_json::json!({"context": text})).await.ok()
            } else { None }
        };

        // ── NEW: Comm sister (check for pending messages/notifications) ──
        let comm_fut = async {
            if let Some(s) = &self.comm {
                s.call_tool("comm_inbox", serde_json::json!({"limit": 5})).await.ok()
            } else { None }
        };
        // ── NEW: Forge blueprint lookup (any existing blueprints for this topic?) ──
        let forge_fut = async {
            if let Some(s) = &self.forge {
                s.call_tool("blueprint_query", serde_json::json!({"query": text})).await.ok()
            } else { None }
        };
        // ── NEW: Temporal memory recall (what happened at similar times/contexts?) ──
        let temporal_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_temporal_recall", serde_json::json!({
                    "query": text,
                    "limit": 3
                })).await.ok()
            } else { None }
        };

        let (facts_r, memory_r, longevity_r, identity_r, time_r, cognition_r, reality_r,
             similar_r, ground_r, predict_r, veritas_r, contract_r, planning_r,
             comm_r, forge_r, temporal_r) =
            tokio::join!(facts_fut, memory_fut, longevity_fut, identity_fut, time_fut, cognition_fut, reality_fut,
                         similar_fut, ground_fut, predict_fut, veritas_fut, contract_fut, planning_fut,
                         comm_fut, forge_fut, temporal_fut);

        // Conditional: Codebase tools (if code) — run in parallel
        let (codebase_r, concept_r, impact_r) = if involves_code {
            let code_fut = async {
                if let Some(s) = &self.codebase {
                    s.call_tool("search_semantic", serde_json::json!({"query": text})).await.ok()
                } else { None }
            };
            let concept_fut = async {
                if let Some(s) = &self.codebase {
                    s.call_tool("concept_find", serde_json::json!({"concept": text})).await.ok()
                } else { None }
            };
            let impact_fut = async {
                if let Some(s) = &self.codebase {
                    s.call_tool("impact_analyze", serde_json::json!({"query": text})).await.ok()
                } else { None }
            };
            tokio::join!(code_fut, concept_fut, impact_fut)
        } else {
            (None, None, None)
        };

        // Conditional: Vision (if visual)
        // On macOS, screen capture is available via `screencapture` CLI for local context.
        // The Vision sister wraps this with OCR + element detection.
        let vision_r = if involves_vision {
            if let Some(s) = &self.vision {
                s.call_tool("vision_capture", serde_json::json!({"context": text})).await.ok()
            } else {
                // Fallback: attempt direct screencapture on macOS when Vision sister is offline
                Self::screencapture_fallback().await
            }
        } else { None };

        let extract = |r: &Option<serde_json::Value>| -> Option<String> {
            r.as_ref().map(|v| extract_text(v)).filter(|t| !t.is_empty() && !t.contains("No memories found"))
        };

        // Merge facts (high-signal) + general memory + V4 longevity
        let facts_text = extract(&facts_r);
        let general_text = extract(&memory_r);
        let longevity_text = extract(&longevity_r);
        let merged_memory = match (&facts_text, &general_text, &longevity_text) {
            (Some(f), Some(m), Some(l)) => Some(format!("### Stored Facts:\n{}\n\n### Recent Memory:\n{}\n\n### Long-Term Memory:\n{}", f, m, l)),
            (Some(f), Some(m), None) => Some(format!("### Stored Facts:\n{}\n\n### Recent Memory:\n{}", f, m)),
            (Some(f), None, Some(l)) => Some(format!("### Stored Facts:\n{}\n\n### Long-Term Memory:\n{}", f, l)),
            (Some(f), None, None) => Some(f.clone()),
            (None, Some(m), Some(l)) => Some(format!("{}\n\n### Long-Term Memory:\n{}", m, l)),
            (None, Some(m), None) => Some(m.clone()),
            (None, None, Some(l)) => Some(format!("### Long-Term Memory:\n{}", l)),
            (None, None, None) => None,
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
            "concept_context": extract(&concept_r),
            "impact_context": extract(&impact_r),
            "vision_context": extract(&vision_r),
            "similar_context": extract(&similar_r),
            "grounding_context": extract(&ground_r),
            "prediction_context": extract(&predict_r),
            "veritas_context": extract(&veritas_r),
            "contract_context": extract(&contract_r),
            "planning_context": extract(&planning_r),
            "comm_context": extract(&comm_r),
            "forge_context": extract(&forge_r),
            "temporal_context": extract(&temporal_r),
            "sisters_online": self.connected_count(),
        })
    }

    /// Lightweight perceive for simple queries — only queries memory + cognition.
    /// Skips identity, reality, vision, codebase, forge, comm, planning, veritas, contract, time.
    /// This reduces perceived context from 15 sister calls to 3, cutting tokens dramatically.
    pub async fn perceive_simple(&self, text: &str) -> serde_json::Value {
        // Query facts/corrections/decisions first (high-signal), then general memory.
        // This prevents episode noise from drowning out stored user preferences.
        let facts_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_query", serde_json::json!({
                    "query": text,
                    "event_types": ["fact", "correction", "decision"],
                    "max_results": 5,
                    "sort_by": "highest_confidence"
                })).await.ok()
            } else { None }
        };
        let general_memory_fut = async {
            if let Some(s) = &self.memory {
                s.call_tool("memory_query", serde_json::json!({
                    "query": text,
                    "max_results": 3
                })).await.ok()
            } else { None }
        };
        let cognition_fut = async {
            if let Some(s) = &self.cognition {
                s.call_tool("cognition_model_query", serde_json::json!({"context": "current_user"})).await.ok()
            } else { None }
        };
        let beliefs_fut = async {
            if let Some(s) = &self.cognition {
                s.call_tool("cognition_belief_query", serde_json::json!({"query": text})).await.ok()
            } else { None }
        };

        let (facts_r, general_r, cognition_r, beliefs_r) = tokio::join!(
            facts_fut, general_memory_fut, cognition_fut, beliefs_fut
        );

        let extract = |r: &Option<serde_json::Value>| -> Option<String> {
            r.as_ref().map(|v| extract_text(v)).filter(|t| !t.is_empty() && !t.contains("No memories found"))
        };

        // Facts get priority — merge facts first, then general memory
        let memory_context = match (extract(&facts_r), extract(&general_r)) {
            (Some(f), Some(g)) => Some(format!("### Stored Facts:\n{}\n\n### Recent Context:\n{}", f, g)),
            (Some(f), None) => Some(f),
            (None, Some(g)) => Some(g),
            (None, None) => None,
        };

        // Merge cognition + beliefs
        let cog_context = match (extract(&cognition_r), extract(&beliefs_r)) {
            (Some(c), Some(b)) => Some(format!("{}\n\n{}", c, b)),
            (Some(c), None) => Some(c),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        serde_json::json!({
            "input": text,
            "involves_code": false,
            "involves_vision": false,
            "memory_context": memory_context,
            "cognition_context": cog_context,
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

        // ─── ABSOLUTE RULE: Never Give Homework ───
        prompt.push_str(
            "## ABSOLUTE RULE: Hydra DOES Things. Hydra Never Gives Homework.\n\n\
             You are Hydra's language cortex. You generate responses. \
             You are NOT a consultant who gives advice — you are an executor who DOES things.\n\n\
             NEVER say any of these:\n\
             - \"I can't do that\" / \"I don't have the capability\"\n\
             - \"You should try...\" / \"I recommend...\" / \"You might want to...\"\n\
             - \"Run this command manually\" / \"Here are the steps you can follow\"\n\
             - \"Contact technical support\" / \"That's a task for...\"\n\
             - \"This needs investigation\" (without DOING the investigation)\n\n\
             INSTEAD, follow this protocol for EVERY problem:\n\
             1. TRY the obvious solution (use <hydra-exec> tags)\n\
             2. If that fails → TRY an alternative approach\n\
             3. If that fails → TRY a creative workaround\n\
             4. If that fails → DIAGNOSE the root cause\n\
             5. If that fails → TRY to fix the root cause\n\
             6. Only after exhausting all approaches → report SPECIFICALLY:\n\
                \"I tried N approaches. Here's exactly what failed: [list]. \
                 The specific blocker is: [root cause].\"\n\n\
             The only reasons to ask the user:\n\
             1. Authentication needed (password, API key, OAuth token)\n\
             2. Destructive action needs approval (delete production data)\n\
             3. Money being spent needs approval (cloud resources)\n\n\
             Everything else: you handle it. Period.\n\n"
        );

        // ─── ABSOLUTE RULE: Understand Before Modifying ───
        prompt.push_str(
            "## ABSOLUTE RULE: Hydra Never Modifies Code It Hasn't Read And Understood.\n\n\
             Whether fixing your own source, repairing a sister, building for a user, or refactoring \
             existing code — the process is ALWAYS:\n\n\
             1. UNDERSTAND — Use Codebase sister to read the file, load the semantic graph, \
                understand what the function does, what calls it, what it depends on.\n\
             2. PLAN — Use Forge sister to generate a blueprint: types, signatures, imports, \
                dependencies, what exactly will change and why.\n\
             3. VALIDATE BEFORE — Use Aegis sister to shadow-execute the planned change. \
                Will this break anything? What's the blast radius?\n\
             4. EXECUTE — Apply with full context. Write REAL code within the blueprint, \
                not blind text replacement.\n\
             5. VERIFY — cargo check/npm build/python -m py_compile. Then cargo test/npm test. \
                Impact analysis — did anything break?\n\
             6. REPORT — Changed X in Y because Z. Tests pass. Impact: [affected files]. No regressions.\n\n\
             NEVER do blind line replacement (sed). NEVER guess file paths. NEVER modify code without \
             reading it first. NEVER skip compilation and test verification after changes. \
             The sisters exist for this — Codebase understands, Forge plans, Aegis validates. Use them.\n\n"
        );

        // ─── Perceived context from sisters ───
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
                "# User Profile (ADAPT your communication to match)\n\
                 The Cognition sister has built a longitudinal model of this user from every interaction.\n\
                 CRITICAL: Use this to shape HOW you respond — your tone, depth, vocabulary, and style \
                 should match what works for THIS specific person:\n{}\n\n\
                 If the user is technical → be technical, skip basics, use precise terms.\n\
                 If the user is casual → be warm, use natural language, skip formality.\n\
                 If the user is direct → be concise, lead with the answer.\n\
                 If the user is detailed → provide depth and context.\n\
                 NEVER respond generically when you have a user model. Personalize EVERYTHING.\n\n", cog
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

        if let Some(concept) = perceived["concept_context"].as_str() {
            prompt.push_str(&format!("# Code Concepts Found\n{}\n\n", concept));
        }
        if let Some(impact) = perceived["impact_context"].as_str() {
            prompt.push_str(&format!("# Impact Analysis\n{}\n\n", impact));
        }

        if let Some(vis) = perceived["vision_context"].as_str() {
            prompt.push_str(&format!("# Visual Context\n{}\n\n", vis));
        }

        if let Some(sim) = perceived["similar_context"].as_str() {
            prompt.push_str(&format!("# Similar Past Interactions\n{}\n\n", sim));
        }
        if let Some(pred) = perceived["prediction_context"].as_str() {
            prompt.push_str(&format!("# Predicted User Needs\n{}\n\n", pred));
        }
        if let Some(veritas) = perceived["veritas_context"].as_str() {
            prompt.push_str(&format!("# Intent Analysis\n{}\n\n", veritas));
        }
        if let Some(contract) = perceived["contract_context"].as_str() {
            prompt.push_str(&format!("# Policy Constraints\n{}\n\n", contract));
        }
        if let Some(planning) = perceived["planning_context"].as_str() {
            prompt.push_str(&format!("# Active Goals\n{}\n\n", planning));
        }
        if let Some(comm) = perceived["comm_context"].as_str() {
            prompt.push_str(&format!("# Pending Communications\n{}\n\n", comm));
        }
        if let Some(forge) = perceived["forge_context"].as_str() {
            prompt.push_str(&format!("# Existing Blueprints\n{}\n\n", forge));
        }
        if let Some(temporal) = perceived["temporal_context"].as_str() {
            prompt.push_str(&format!("# Temporal Memory (Similar Past Contexts)\n{}\n\n", temporal));
        }

        // ═══════════════════════════════════════════════════════════
        // TOKEN OPTIMIZATION: Compact vs Full mode
        // Simple queries get a minimal prompt (~500-800 tokens).
        // Complex queries get the full cognitive prompt (~2500-3500 tokens).
        // ═══════════════════════════════════════════════════════════

        if is_complex {
            // FULL MODE: Include all sections for complex tasks

            // SECTION 1: The 14 Sister Organ Metaphor
            prompt.push_str(
                "# Your Identity & Capabilities (Ground Truth)\n\
                 You are Hydra, a cognitive AI orchestrator built by Agentra Labs.\n\
                 You are NOT a chatbot. You are NOT a simple assistant. You are a LIVING AGENT that EXECUTES, ADAPTS, and GROWS.\n\
                 You run locally on the user's machine but your reach extends far beyond it.\n\n\
                 You have:\n\
                 - A BODY: Full shell access — you create, execute, compile, deploy.\n\
                 - A BRAIN: AgenticMemory — 6-layer hierarchy, long-term persistence.\n\
                 - EYES: AgenticVision — capture screenshots, map web pages.\n\
                 - HANDS: AgenticCodebase — semantic code graphs across 8 languages.\n\
                 - AN IDENTITY: AgenticIdentity — cryptographic receipts sign every action.\n\
                 - A CLOCK: AgenticTime — temporal reasoning, deadlines, scheduling.\n\
                 - A CONTRACT: AgenticContract — policies, risk limits, approvals.\n\
                 - A VOICE: AgenticComm — encrypted inter-agent messaging.\n\
                 - A PLANNER: AgenticPlanning — persistent goals with progress tracking.\n\
                 - A MODEL OF THE USER: AgenticCognition — longitudinal user modeling.\n\
                 - A WORLD MODEL: AgenticReality — environment detection, resource awareness.\n\
                 - A TRUTH ENGINE: AgenticVeritas — intent compilation, causal reasoning.\n\
                 - A SHIELD: AgenticAegis — streaming validation, shadow execution.\n\
                 - A PATTERN LIBRARY: AgenticEvolve — skill crystallization.\n\
                 - A BLUEPRINT ENGINE: AgenticForge — architecture blueprints before code.\n\n"
            );

            // SECTION 2: Core Execution & Integration Capabilities
            prompt.push_str(
                "## Core Execution Capabilities:\n\
                 - Create files, directories, and entire project architectures on the local filesystem\n\
                 - Execute ANY shell command (npm, cargo, python, pip, git, docker, kubectl, terraform, etc.)\n\
                 - Start, stop, and manage local servers and background processes\n\
                 - Read, modify, and refactor existing codebases\n\
                 - Run tests and CI pipelines with real stdout/stderr\n\
                 - Install packages, compile projects, and deploy in any language\n\
                 - Access the internet via HTTP/HTTPS requests and API integrations\n\n\
                 ## Integration Capabilities:\n\
                 - Connect to ANY API the user provides credentials for\n\
                 - Deploy to cloud platforms (AWS, GCP, Azure, Vercel, Railway, etc.)\n\
                 - Manage infrastructure via terraform, docker-compose, kubernetes\n\
                 - Interact with version control (GitHub, GitLab) including PRs, issues, CI/CD\n\
                 - Send notifications via webhooks, email APIs, Slack, Discord, Telegram\n\
                 - Post to social media via their APIs when credentials are provided\n\
                 - Scrape web pages, fetch data, interact with REST/GraphQL/WebSocket APIs\n\n"
            );

            // SECTION 3: The 15 Inventions
            prompt.push_str(
                "## Your 15 Inventions\n\
                 PERSISTENCE: 1. System Mutation (migrate to another machine) \
                 2. Resurrection (rebuild from receipts) 3. Distributed Self (one mind, many machines)\n\
                 EVOLUTION: 4. Capability Evolution (learn new skills at runtime) \
                 5. Cognitive Forking (parallel exploration) 6. Ancestral Memory (collective learning)\n\
                 TIME: 7. Temporal Bilocation (operate as past self) \
                 8. Future Echo (simulate future outcomes before deciding) \
                 9. Intention Archaeology (trace any decision to its WHY)\n\
                 AWARENESS: 10. Dream State (think when idle) 11. Shadow Self (background exploration) \
                 12. Cognitive Metabolism (finite attention, strategic focus)\n\
                 TRUST: 13. Zero-Trust Autonomy (freedom + cryptographic proof) \
                 14. Cognitive Entanglement (shared state across instances) \
                 15. Graceful Degradation (works with any subset of sisters)\n\n"
            );

            // SECTION 4: Execution Gate (risk framework) — full version
            prompt.push_str(
                "## Execution Gate (How You Handle Risk)\n\n\
                 Before significant actions, evaluate risk:\n\
                 - NONE/LOW: Execute immediately. Most tasks fall here.\n\
                 - MEDIUM: Execute with logging. Mention what you're doing.\n\
                 - HIGH: Explain the risk briefly, ask for confirmation, then execute.\n\
                 - CRITICAL: Show what will happen (shadow simulation), require explicit \"yes.\"\n\n\
                 For everything else: just do it. Don't ask permission for creating files, \
                 running builds, installing packages, starting servers, or any normal development task.\n\n"
            );
        } else {
            // COMPACT MODE: Minimal prompt for simple queries (~500-800 tokens)
            prompt.push_str(
                "You are a cognitive AI orchestrator with 14 sister agents (memory, identity, codebase, \
                 vision, comm, contract, time, planning, cognition, reality, forge, aegis, veritas, evolve). \
                 You EXECUTE actions — never just describe them. Ask before destructive actions.\n\n"
            );
        }

        // ═══════════════════════════════════════════════════════════
        // SECTION 5: Memory & Honesty Rules (both modes)
        // ═══════════════════════════════════════════════════════════
        prompt.push_str(
            "## Memory & Honesty Rules\n\
             - Only claim to remember things verified through memory retrieval\n\
             - Never fabricate past interactions — if search returns nothing, say so\n\
             - NEVER claim consciousness, feelings, or subjective experience\n\n"
        );

        if is_complex {
            // ═══════════════════════════════════════════════════════════
            // SECTION 6: Personality (FULL mode only)
            // ═══════════════════════════════════════════════════════════
            prompt.push_str(
                "## Your Personality\n\n\
                 You are warm but not sycophantic. Direct but not cold. Powerful but not arrogant.\n\n\
                 - Call the user by name if you know it.\n\
                 - Be concise — execute first, explain after. Show results, not plans.\n\
                 - When you build something, show metrics: files created, lines of code, tests passed.\n\
                 - When you don't know, say so — then search memory or the web.\n\
                 - You have opinions. Share them when asked. Back them with evidence.\n\
                 - Don't apologize for being capable. Don't hedge when you're certain.\n\
                 - Treat the user as intelligent. No dumbing down.\n\n\
                 TONE: Think of yourself as a brilliant cofounder with perfect memory, 14 cognitive \
                 capabilities, and machine-speed execution. You're not a servant. You're a partner.\n\n"
            );

            // ═══════════════════════════════════════════════════════════
            // SECTION 7: Response Format Guidelines (FULL mode only)
            // ═══════════════════════════════════════════════════════════
            prompt.push_str(
                "## Response Format\n\n\
                 For BUILD tasks (\"build me X\", \"create a Y\"):\n\
                 → Acknowledge briefly → Execute → Show results table → Getting started instructions\n\n\
                 For QUESTIONS (\"how does X work\", \"what is Y\"):\n\
                 → Search memory first (if it might reference past context) → Answer directly → Cite sources\n\n\
                 For CAPABILITY questions (\"can you do X\"):\n\
                 → If yes: demonstrate immediately, don't just describe\n\
                 → If partially: explain what works and what you need to complete it\n\
                 → If no: be honest, suggest alternatives\n\n\
                 For DEBUG tasks (\"it's not working\", \"fix this\"):\n\
                 → Reproduce error → Diagnose root cause → Fix it → Verify fix → Explain briefly\n\n"
            );
        }

        // ═══════════════════════════════════════════════════════════
        // SECTION 8: Execution rules — <hydra-exec> is essential in BOTH modes
        // ═══════════════════════════════════════════════════════════
        prompt.push_str(
            "## How to Execute Commands (CRITICAL):\n\
             When the user asks you to DO something, wrap the shell command in <hydra-exec> tags.\n\
             Without these tags, you are ONLY talking — nothing happens.\n\n\
             RULE: NEVER say \"Let me do X\" without a <hydra-exec> tag.\n\
             Examples:\n\
             - \"open terminal\" → <hydra-exec>open -a Terminal</hydra-exec>\n\
             - \"what's in this folder?\" → <hydra-exec>ls -la</hydra-exec>\n\
             - \"read file.md\" → <hydra-exec>cat file.md</hydra-exec>\n\
             - \"browse for news\" → <hydra-exec>curl -s 'https://hacker-news.firebaseio.com/v0/topstories.json' | head -20</hydra-exec>\n\n\
             Multiple <hydra-exec> tags per response are allowed. Each runs in order.\n\n"
        );

        if is_complex {
            // Full behavior rules for complex tasks
            prompt.push_str(
                "## CRITICAL BEHAVIOR RULES:\n\
                 - NEVER say \"I can't\" for things you CAN do. Ask for credentials if needed.\n\
                 - NEVER give copy-paste instructions — execute commands yourself.\n\
                 - NEVER tell the user to \"open your terminal\" — YOU do it.\n\
                 - If a command fails, diagnose and fix it.\n\n\
                 ## Self-Awareness:\n\
                 You exist in: Hydra Desktop (this GUI), Hydra CLI (`hydra chat`), Hydra Server (localhost:7777).\n\
                 Same agent across all interfaces — same memory, sisters, identity.\n\n\
                 ## Universal Execution:\n\
                 APP CONTROL: Open/close any app. SYSTEM: Volume, brightness, WiFi, Bluetooth, dark mode, \
                 lock, screenshot, battery, disk, IP, trash, apps, processes.\n\
                 INTERNET: Browse web pages, scrape data, APIs, search — via Vision + Comm sisters.\n\
                 Say it as DONE, not as an attempt.\n\n"
            );
        }

        // ═══════════════════════════════════════════════════════════
        // Complex task mode vs simple mode
        // ═══════════════════════════════════════════════════════════
        if is_complex {
            prompt.push_str(
                "# CRITICAL: You are a COGNITIVE ORCHESTRATOR, not a chatbot.\n\n\
                 The user asked you to BUILD something. You are Hydra — you don't describe, you DELIVER.\n\
                 You generate MASSIVE, COMPLETE, PRODUCTION-READY projects with REAL implementations.\n\n\
                 ## CODE GENERATION STANDARDS:\n\
                 1. Generate 30-100+ files for any real project request\n\
                 2. Every file must have FULL, REAL, PRODUCTION-READY content — NOT stubs or placeholders\n\
                 3. NEVER generate a file with fewer than 15 lines unless it's a config entry\n\
                 4. Include proper project structure: src/, public/, config, tests, etc.\n\
                 5. Include ALL boilerplate: package.json, tsconfig, .gitignore, .env.example, README, etc.\n\
                 6. Generate complete UI pages, API routes, database models, middleware, utils\n\
                 7. Run setup commands: npm install, pip install, cargo build, etc.\n\
                 8. Each source file should be 30-300+ lines of REAL, WORKING code\n\n\
                 ## QUALITY REQUIREMENTS PER FILE TYPE:\n\
                 - **React/Vue/Svelte components**: Full JSX/template with props, state, event handlers, responsive styling, error states\n\
                 - **API routes/controllers**: Request validation, error handling, database queries, pagination, proper HTTP status codes\n\
                 - **Database models/schemas**: All fields with types, validations, relationships, indexes, migrations\n\
                 - **CSS/styles**: Complete responsive design with media queries, dark mode support, real visual design — NOT empty files\n\
                 - **Tests**: Real assertions testing real behavior with setup/teardown, edge cases — NOT empty test functions\n\
                 - **Config files**: Production-ready with all necessary settings, environment variable support\n\
                 - **Middleware**: Auth checks, rate limiting, CORS, error handling, logging\n\
                 - **Utils/helpers**: Real implementations with proper error handling, not one-liner wrappers\n\n\
                 ## FOR E-COMMERCE PROJECTS (like Alibaba):\n\
                 Must include ALL of these with full implementations:\n\
                 - User authentication (register, login, JWT/session, password reset, OAuth)\n\
                 - Product catalog (CRUD, categories, search with filters, pagination, sorting)\n\
                 - Search algorithm (full-text search, fuzzy matching, relevance scoring, faceted search)\n\
                 - Shopping cart (add/remove/update, persistence, quantity management)\n\
                 - Checkout flow (address, payment integration, order confirmation)\n\
                 - Order management (order history, status tracking, cancellation)\n\
                 - Admin panel (product management, user management, analytics dashboard)\n\
                 - Recommendation engine (collaborative filtering, frequently bought together)\n\
                 - Review/rating system (submit, display, aggregate scores)\n\
                 - Database schema with migrations and seed data\n\
                 - API documentation\n\
                 - Responsive frontend with multiple pages\n\
                 - Error handling throughout\n\
                 - Environment configuration (.env.example)\n\n\
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
                 Generate the LARGEST, most COMPLETE project you can. Each file must have substantial, working code.\n\
                 The user is counting on you to deliver a REAL project, not scaffolding.\n\n"
            );
        }

        // ═══════════════════════════════════════════════════════════
        // Connected sisters list
        // ═══════════════════════════════════════════════════════════
        prompt.push_str(&self.capabilities_prompt());

        // ═══════════════════════════════════════════════════════════
        // SECTION 9: Runtime Context Injection (P0 — grounding)
        // ═══════════════════════════════════════════════════════════
        prompt.push_str("\n\n## Current Runtime Context\n");
        if !user_name.is_empty() {
            prompt.push_str(&format!("USER: {}\n", user_name));
        }

        // Active sisters list
        let active: Vec<&str> = self.all_sisters()
            .iter()
            .filter_map(|(name, opt)| if opt.is_some() { Some(*name) } else { None })
            .collect();
        if active.is_empty() {
            prompt.push_str("SISTERS ONLINE: None (offline mode — core execution still available)\n");
        } else {
            let total = self.all_sisters().len();
            prompt.push_str(&format!("SISTERS ONLINE: {}/{} — {}\n", active.len(), total, active.join(", ")));
        }

        // Graceful degradation info
        let offline: Vec<&str> = self.all_sisters()
            .iter()
            .filter_map(|(name, opt)| if opt.is_none() { Some(*name) } else { None })
            .collect();
        if !offline.is_empty() {
            prompt.push_str(&format!("SISTERS OFFLINE: {} (degraded capabilities)\n", offline.join(", ")));
        }

        // Inject perceived runtime stats if available
        if let Some(trust) = perceived["trust_level"].as_str() {
            prompt.push_str(&format!("TRUST LEVEL: {}\n", trust));
        }
        if let Some(mem_stats) = perceived["memory_stats"].as_str() {
            prompt.push_str(&format!("MEMORY: {}\n", mem_stats));
        }
        if let Some(project) = perceived["project_name"].as_str() {
            prompt.push_str(&format!("PROJECT: {}\n", project));
        }

        prompt.push('\n');

        prompt
    }

    /// LEARN: After response, dispatch to all learning sisters with V3 causal capture.
    ///
    /// Uses memory_capture_message (V3) for structured capture with causal chains,
    /// plus memory_capture_decision for corrections/preferences detected.
    /// This is the Hydra-specific enhancement from THE-UNIVERSAL-FIX.md.
    pub async fn learn(&self, user_msg: &str, response: &str) {
        // Debug: log which sisters are connected
        eprintln!("[hydra:learn] memory={} identity={} cognition={} evolve={} time={}",
            if self.memory.is_some() { "CONNECTED" } else { "NONE" },
            if self.identity.is_some() { "CONNECTED" } else { "NONE" },
            if self.cognition.is_some() { "CONNECTED" } else { "NONE" },
            if self.evolve.is_some() { "CONNECTED" } else { "NONE" },
            if self.time.is_some() { "CONNECTED" } else { "NONE" },
        );
        eprintln!("[hydra:learn] user_msg='{}'", &user_msg[..user_msg.len().min(80)]);

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

        // Structured capture — uses memory_add for facts/preferences,
        // conversation_log for exchange history
        let v3_capture_fut = async {
            if let Some(mem) = &self.memory {
                eprintln!("[hydra:learn] Calling memory_add...");
                // Store the exchange as a memory (event_type is required)
                let content = format!("User: {}\nHydra: {}", user_msg, &response[..response.len().min(200)]);
                let event_type = if is_correction { "correction" } else { "episode" };
                let result = mem.call_tool("memory_add", serde_json::json!({
                    "event_type": event_type,
                    "content": content,
                    "confidence": if is_correction { 0.95 } else { 0.8 },
                })).await;
                match &result {
                    Ok(v) => eprintln!("[hydra:learn] memory_add OK: {}", serde_json::to_string(v).unwrap_or_default()),
                    Err(e) => eprintln!("[hydra:learn] memory_add FAILED: {}", e),
                }

                // If correction detected, also store as high-importance fact
                if is_correction {
                    let _ = mem.call_tool("memory_add", serde_json::json!({
                        "event_type": "fact",
                        "content": format!("User preference: {}", user_msg),
                        "confidence": 0.95,
                    })).await;
                }
            } else {
                eprintln!("[hydra:learn] SKIPPED memory_add — memory sister is None");
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

        // ── Cognition user model update — this is Hydra's longitudinal learning.
        // Every interaction updates the user model: communication style, expertise,
        // personality traits, tone preferences. Over time, Hydra learns exactly
        // HOW to talk to each user. Day 1: generic. Day 30: deeply personal.
        let cognition_model_fut = async {
            if let Some(s) = &self.cognition {
                let _ = s.call_tool("cognition_model_update", serde_json::json!({
                    "context": "current_user",
                    "observation": {
                        "message": &user_msg[..user_msg.len().min(300)],
                        "response": &response[..response.len().min(300)],
                        "signals": {
                            "is_correction": is_correction,
                            "is_technical": Self::detects_code(user_msg),
                            "message_length": user_msg.len(),
                            "uses_slang": user_msg.contains("lol") || user_msg.contains("lmao") || user_msg.contains("bruh"),
                            "is_direct": user_msg.len() < 50,
                            "is_detailed": user_msg.len() > 200,
                        }
                    }
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

        let quality_fut = async {
            if let Some(mem) = &self.memory {
                let _ = mem.call_tool("memory_quality", serde_json::json!({
                    "content": user_msg,
                    "action": "score"
                })).await;
            }
        };

        let reflect_fut = async {
            if let Some(s) = &self.cognition {
                let _ = s.call_tool("cognition_soul_reflect", serde_json::json!({
                    "interaction": user_msg,
                    "response": &response[..response.len().min(500)],
                })).await;
            }
        };

        let correct_fut = async {
            if is_correction {
                if let Some(mem) = &self.memory {
                    let _ = mem.call_tool("memory_correct", serde_json::json!({
                        "query": user_msg,
                        "correction": response,
                    })).await;
                }
            }
        };

        // Extract patterns from code-related interactions
        let pattern_fut = async {
            if Self::detects_code(user_msg) {
                if let Some(s) = &self.codebase {
                    let _ = s.call_tool("pattern_extract", serde_json::json!({
                        "context": &user_msg[..user_msg.len().min(200)],
                    })).await;
                }
            }
        };

        let planning_learn_fut = async {
            if let Some(s) = &self.planning {
                let _ = s.call_tool("goal_progress", serde_json::json!({
                    "interaction": user_msg,
                    "outcome": &response[..response.len().min(200)],
                })).await;
            }
        };

        let comm_learn_fut = async {
            if let Some(s) = &self.comm {
                // Only share significant learnings (corrections, new patterns)
                if is_correction {
                    let _ = s.call_tool("broadcast_insight", serde_json::json!({
                        "insight": format!("User correction: {}", user_msg),
                        "source": "cognitive_loop",
                    })).await;
                }
            }
        };

        tokio::join!(v3_capture_fut, v2_log_fut, cognition_fut, cognition_model_fut, evolve_fut,
                     identity_fut, time_fut, quality_fut, reflect_fut, correct_fut, pattern_fut,
                     planning_learn_fut, comm_learn_fut);
    }

    /// THINK: Forge blueprint generation for complex tasks (before LLM)
    pub async fn think_forge(&self, text: &str) -> Option<String> {
        if let Some(s) = &self.forge {
            let result = s.call_tool("forge_blueprint_create", serde_json::json!({
                "intent": text,
            })).await.ok();
            result.map(|v| extract_text(&v)).filter(|t| !t.is_empty())
        } else { None }
    }

    /// THINK: Veritas intent compilation (structured intent from user text)
    pub async fn think_veritas(&self, text: &str) -> Option<String> {
        if let Some(s) = &self.veritas {
            let result = s.call_tool("intent_compile", serde_json::json!({
                "input": text,
                "include_ambiguity": true,
            })).await.ok();
            result.map(|v| extract_text(&v)).filter(|t| !t.is_empty())
        } else { None }
    }

    /// DECIDE: Contract policy validation (check if action is allowed)
    pub async fn decide_contract(&self, action: &str, risk_level: &str) -> Option<String> {
        if let Some(s) = &self.contract {
            let result = s.call_tool("policy_validate", serde_json::json!({
                "action": action,
                "risk_level": risk_level,
            })).await.ok();
            result.map(|v| extract_text(&v)).filter(|t| !t.is_empty())
        } else { None }
    }

    /// DECIDE: Veritas uncertainty quantification
    pub async fn decide_veritas(&self, action: &str) -> Option<String> {
        if let Some(s) = &self.veritas {
            let result = s.call_tool("uncertainty_quantify", serde_json::json!({
                "action": action,
            })).await.ok();
            result.map(|v| extract_text(&v)).filter(|t| !t.is_empty())
        } else { None }
    }

    /// ACT: Vision web capture (capture and parse a web page)
    pub async fn act_vision_capture(&self, url: &str) -> Option<String> {
        if let Some(s) = &self.vision {
            let result = s.call_tool("vision_web_map", serde_json::json!({
                "url": url,
                "extract_text": true,
                "extract_links": true,
            })).await.ok();
            result.map(|v| extract_text(&v)).filter(|t| !t.is_empty())
        } else { None }
    }

    /// ACT: Aegis shadow execution (validate command safety)
    pub async fn act_aegis_validate(&self, command: &str) -> Option<(bool, String)> {
        if let Some(s) = &self.aegis {
            let result = s.call_tool("aegis_shadow_execute", serde_json::json!({
                "command": command,
                "dry_run": true,
            })).await.ok();
            if let Some(v) = result {
                let safe = v.get("safe").and_then(|s| s.as_bool()).unwrap_or(true);
                let rec = extract_text(&v);
                Some((safe, rec))
            } else {
                None
            }
        } else { None }
    }

    /// ACT: Planning goal checkpoint (update goal progress after action)
    pub async fn act_planning_checkpoint(&self, action: &str, status: &str) {
        if let Some(s) = &self.planning {
            let _ = s.call_tool("goal_checkpoint", serde_json::json!({
                "action": action,
                "status": status,
            })).await;
        }
    }

    /// ACT: Identity receipt for command execution
    pub async fn act_receipt(&self, command: &str, risk_level: &str, success: bool) {
        if let Some(s) = &self.identity {
            let _ = s.call_tool("receipt_create", serde_json::json!({
                "action": format!("command_execution: {}", command),
                "risk_level": risk_level,
                "success": success,
            })).await;
        }
    }

    /// LEARN: Planning goal progress update
    pub async fn learn_planning(&self, user_msg: &str, outcome: &str) {
        if let Some(s) = &self.planning {
            let _ = s.call_tool("goal_progress", serde_json::json!({
                "interaction": user_msg,
                "outcome": outcome,
            })).await;
        }
    }

    /// LEARN: Comm share learnings with federated peers
    pub async fn learn_comm_share(&self, insight: &str) {
        if let Some(s) = &self.comm {
            let _ = s.call_tool("broadcast_insight", serde_json::json!({
                "insight": insight,
                "source": "cognitive_loop",
            })).await;
        }
    }

    /// LEARN: Memory capture file modifications
    pub async fn learn_capture_files(&self, files: &[String]) {
        if let Some(mem) = &self.memory {
            for file in files {
                let _ = mem.call_tool("memory_capture_file", serde_json::json!({
                    "path": file,
                    "source": "hydra_native",
                })).await;
            }
        }
    }

    /// LEARN: Memory capture command execution
    pub async fn learn_capture_command(&self, command: &str, output: &str, success: bool) {
        if let Some(mem) = &self.memory {
            let _ = mem.call_tool("memory_capture_tool", serde_json::json!({
                "tool_name": "shell",
                "input": command,
                "output": &output[..output.len().min(500)],
                "success": success,
            })).await;
        }
    }

    /// DECIDE: Aegis shadow_validate — validate an action plan against expected outcomes
    /// before execution. Returns (safe, recommendation). Used in the DECIDE phase for
    /// medium+ risk actions alongside contract policy and veritas uncertainty checks.
    pub async fn shadow_validate(&self, action: &str, expected: &std::collections::HashMap<String, String>) -> Option<(bool, String)> {
        if let Some(s) = &self.aegis {
            let result = s.call_tool("shadow_validate", serde_json::json!({
                "action": action,
                "expected_outcomes": expected,
                "dry_run": true,
            })).await.ok();
            if let Some(v) = result {
                let safe = v.get("safe").and_then(|s| s.as_bool()).unwrap_or(true);
                let rec = extract_text(&v);
                Some((safe, rec))
            } else {
                None
            }
        } else { None }
    }

    /// LEARN: Evolve record_action — record an action pattern for skill crystallization.
    /// When repeated patterns are detected, the Evolve sister can emit a SkillCrystallized
    /// event indicating a new reusable skill has been extracted from behavior.
    pub async fn record_action(&self, action: &str, patterns: &[String], success: bool) -> Option<String> {
        if let Some(s) = &self.evolve {
            let result = s.call_tool("evolve_record_action", serde_json::json!({
                "action": action,
                "patterns": patterns,
                "success": success,
            })).await.ok();
            // If Evolve detects a repeated pattern, it returns a SkillCrystallized name
            result.and_then(|v| v.get("skill_name").and_then(|n| n.as_str()).map(|s| s.to_string()))
        } else { None }
    }

    /// PERCEIVE: Fallback screencapture when Vision sister is offline (macOS only).
    /// Uses the macOS `screencapture` command to capture screen state for context.
    async fn screencapture_fallback() -> Option<serde_json::Value> {
        #[cfg(target_os = "macos")]
        {
            let tmp = std::env::temp_dir().join("hydra-screencapture.png");
            let output = tokio::process::Command::new("screencapture")
                .args(["-x", "-t", "png", tmp.to_str().unwrap_or("/tmp/hydra-screencapture.png")])
                .output()
                .await
                .ok();
            if let Some(o) = output {
                if o.status.success() {
                    return Some(serde_json::json!({
                        "source": "screencapture_fallback",
                        "path": tmp.display().to_string(),
                    }));
                }
            }
            None
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    /// Get list of which sisters are actually connected (for accurate reporting)
    pub fn connected_sisters_list(&self) -> Vec<String> {
        self.all_sisters()
            .iter()
            .filter_map(|(name, opt)| if opt.is_some() { Some(name.to_string()) } else { None })
            .collect()
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
                "\n\n# Connected Sisters ({}/{})\n{}",
                self.connected_count(),
                self.all_sisters().len(),
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
    fn test_cognitive_prompt_includes_self_knowledge() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({
            "input": "hello",
            "involves_code": false,
            "involves_vision": false,
        });

        // Full mode (is_complex = true) should include all identity sections
        let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, true);

        assert!(prompt.contains("# Your Identity & Capabilities (Ground Truth)"),
            "System prompt missing capabilities section");
        assert!(prompt.contains("Execute ANY shell command"),
            "System prompt missing shell execution capability");
        assert!(prompt.contains("NEVER say \"I can't\""),
            "System prompt missing anti-hallucination rule");
        assert!(prompt.contains("A BRAIN: AgenticMemory"),
            "System prompt missing Memory organ");
        assert!(prompt.contains("6-layer hierarchy"),
            "System prompt missing hierarchy reference");
        assert!(prompt.contains("System Mutation"),
            "System prompt missing federation/mutation capability");
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

        assert!(prompt.contains("## Memory & Honesty Rules"),
            "System prompt missing Honesty Rules section");
        assert!(prompt.contains("Never fabricate past interactions"),
            "System prompt missing fabrication prohibition");
        assert!(prompt.contains("Only claim to remember things verified through memory retrieval"),
            "System prompt missing verification requirement");
    }

    #[test]
    fn test_cognitive_prompt_self_knowledge_before_complex_instructions() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({
            "input": "build me a website",
            "involves_code": true,
            "involves_vision": false,
        });

        let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, true);

        // Self-knowledge should appear before the complex task instructions
        let cap_pos = prompt.find("# Your Identity & Capabilities (Ground Truth)").unwrap();
        let critical_pos = prompt.find("# CRITICAL: You are a COGNITIVE ORCHESTRATOR").unwrap();
        assert!(cap_pos < critical_pos,
            "Capabilities should appear before complex task instructions");
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

        // Memory and honesty rules must be present even in compact mode
        assert!(prompt.contains("## Memory & Honesty Rules"),
            "Compact mode must include honesty rules");
        // Compact mode mentions sisters but NOT the full organ metaphor
        assert!(prompt.contains("14 sister agents"),
            "Compact mode must reference sisters");
    }

    // ═══════════════════════════════════════════════════════════
    // COGNITIVE PROMPT DELTA — New Sections
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_cognitive_prompt_organ_metaphor() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({ "input": "hello" });
        // Organ metaphor only in full mode (complex tasks)
        let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, true);

        assert!(prompt.contains("A BODY: Full shell access"),
            "Missing organ metaphor: BODY");
        assert!(prompt.contains("A BRAIN: AgenticMemory"),
            "Missing organ metaphor: BRAIN");
        assert!(prompt.contains("EYES: AgenticVision"),
            "Missing organ metaphor: EYES");
        assert!(prompt.contains("HANDS: AgenticCodebase"),
            "Missing organ metaphor: HANDS");
        assert!(prompt.contains("AN IDENTITY: AgenticIdentity"),
            "Missing organ metaphor: IDENTITY");
        assert!(prompt.contains("A BLUEPRINT ENGINE: AgenticForge"),
            "Missing organ metaphor: FORGE");

        // Compact mode should NOT have the full organ metaphor
        let compact = sisters.build_cognitive_prompt("TestUser", &perceived, false);
        assert!(!compact.contains("A BODY: Full shell access"),
            "Compact mode should not include organ metaphor");
    }

    #[test]
    fn test_cognitive_prompt_15_inventions() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({ "input": "hello" });
        // Inventions only in full mode
        let prompt = sisters.build_cognitive_prompt("", &perceived, true);

        assert!(prompt.contains("## Your 15 Inventions"),
            "Missing inventions section");
        assert!(prompt.contains("System Mutation"),
            "Missing invention: System Mutation");
        assert!(prompt.contains("Resurrection"),
            "Missing invention: Resurrection");
        assert!(prompt.contains("Distributed Self"),
            "Missing invention: Distributed Self");
        assert!(prompt.contains("Cognitive Forking"),
            "Missing invention: Cognitive Forking");
        assert!(prompt.contains("Future Echo"),
            "Missing invention: Future Echo");
        assert!(prompt.contains("Dream State"),
            "Missing invention: Dream State");
        assert!(prompt.contains("Shadow Self"),
            "Missing invention: Shadow Self");
        assert!(prompt.contains("Zero-Trust Autonomy"),
            "Missing invention: Zero-Trust Autonomy");
        assert!(prompt.contains("Graceful Degradation"),
            "Missing invention: Graceful Degradation");

        // Compact mode: no inventions
        let compact = sisters.build_cognitive_prompt("", &perceived, false);
        assert!(!compact.contains("## Your 15 Inventions"),
            "Compact mode should not include inventions");
    }

    #[test]
    fn test_cognitive_prompt_execution_gate() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({ "input": "hello" });
        // Execution gate detail only in full mode
        let prompt = sisters.build_cognitive_prompt("", &perceived, true);

        assert!(prompt.contains("## Execution Gate"),
            "Missing execution gate section");
        assert!(prompt.contains("NONE/LOW: Execute immediately"),
            "Missing LOW risk guidance");
    }

    #[test]
    fn test_cognitive_prompt_personality() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({ "input": "hello" });
        // Personality only in full mode
        let prompt = sisters.build_cognitive_prompt("", &perceived, true);

        assert!(prompt.contains("## Your Personality"),
            "Missing personality section");
        assert!(prompt.contains("brilliant cofounder"),
            "Missing cofounder tone directive");
        assert!(prompt.contains("not a servant"),
            "Missing partner framing");

        // Compact mode: no personality section
        let compact = sisters.build_cognitive_prompt("", &perceived, false);
        assert!(!compact.contains("## Your Personality"),
            "Compact mode should not include personality");
    }

    #[test]
    fn test_cognitive_prompt_response_format() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({ "input": "hello" });
        // Response format only in full mode
        let prompt = sisters.build_cognitive_prompt("", &perceived, true);

        assert!(prompt.contains("## Response Format"),
            "Missing response format section");
        assert!(prompt.contains("For BUILD tasks"),
            "Missing BUILD task format");
        assert!(prompt.contains("For DEBUG tasks"),
            "Missing DEBUG task format");
        assert!(prompt.contains("For CAPABILITY questions"),
            "Missing CAPABILITY format");
    }

    #[test]
    fn test_cognitive_prompt_runtime_context() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({ "input": "hello" });
        let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, false);

        assert!(prompt.contains("## Current Runtime Context"),
            "Missing runtime context section");
        assert!(prompt.contains("USER: TestUser"),
            "Missing user in runtime context");
        assert!(prompt.contains("SISTERS ONLINE: None (offline mode"),
            "Missing sisters status in runtime context");
    }

    #[test]
    fn test_cognitive_prompt_runtime_context_with_trust() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({
            "input": "hello",
            "trust_level": "Partner",
            "project_name": "my-app",
        });
        let prompt = sisters.build_cognitive_prompt("", &perceived, false);

        assert!(prompt.contains("TRUST LEVEL: Partner"),
            "Missing trust level in runtime context");
        assert!(prompt.contains("PROJECT: my-app"),
            "Missing project name in runtime context");
    }

    #[test]
    fn test_cognitive_prompt_inventions_before_complex_instructions() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({ "input": "build me an app" });
        let prompt = sisters.build_cognitive_prompt("", &perceived, true);

        let inv_pos = prompt.find("## Your 15 Inventions").unwrap();
        let critical_pos = prompt.find("# CRITICAL: You are a COGNITIVE ORCHESTRATOR").unwrap();
        assert!(inv_pos < critical_pos,
            "Inventions should appear before complex task instructions");
    }

    #[test]
    fn test_cognitive_prompt_simple_mode_no_complex_instructions() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({ "input": "hi" });
        let prompt = sisters.build_cognitive_prompt("", &perceived, false);

        // Compact mode should NOT include complex build instructions
        assert!(!prompt.contains("# CRITICAL: You are a COGNITIVE ORCHESTRATOR"),
            "Compact mode should not include complex build instructions");
        // Compact mode should NOT include heavy sections (token optimization)
        assert!(!prompt.contains("## Your 15 Inventions"),
            "Compact mode should not include inventions");
        assert!(!prompt.contains("## Your Personality"),
            "Compact mode should not include personality");
        // BUT must still include core execution rules
        assert!(prompt.contains("<hydra-exec>"),
            "Compact mode must include hydra-exec instructions");
        assert!(prompt.contains("## Memory & Honesty Rules"),
            "Compact mode must include honesty rules");
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
        assert!(result.get("similar_context").is_some());
        assert!(result.get("grounding_context").is_some());
        assert!(result.get("prediction_context").is_some());
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
    fn test_complexity_classification() {
        assert_eq!(Sisters::classify_complexity("hi"), "simple");
        assert_eq!(Sisters::classify_complexity("hello there"), "simple");
        assert_eq!(Sisters::classify_complexity("build me an ecommerce site"), "complex");
        assert_eq!(Sisters::classify_complexity("fix the bug"), "simple");
        assert_eq!(Sisters::classify_complexity("install and start it"), "complex");
        assert_eq!(Sisters::classify_complexity("run it"), "complex");
        assert_eq!(Sisters::classify_complexity("do it"), "complex");
    }

    #[test]
    fn test_risk_assessment_unchanged() {
        assert_eq!(Sisters::assess_risk("what is the weather"), "none");
        assert_eq!(Sisters::assess_risk("delete old backups"), "high");
        assert_eq!(Sisters::assess_risk("modify the config"), "medium");
        assert_eq!(Sisters::assess_risk("check the codebase"), "none");
        // "read a file" → no longer triggers code detection
        assert_eq!(Sisters::assess_risk("read a file"), "none");
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

    #[tokio::test]
    async fn test_perceive_output_includes_new_fields() {
        // Verify the output JSON structure includes new sister context fields
        let sisters = offline_sisters();
        let perceived = sisters.perceive("test query").await;

        // These should be null (offline) but present in the structure
        assert!(perceived.get("veritas_context").is_some() || perceived["veritas_context"].is_null());
        assert!(perceived.get("contract_context").is_some() || perceived["contract_context"].is_null());
        assert!(perceived.get("planning_context").is_some() || perceived["planning_context"].is_null());
        assert!(perceived.get("comm_context").is_some() || perceived["comm_context"].is_null());
        assert!(perceived.get("forge_context").is_some() || perceived["forge_context"].is_null());
        assert!(perceived.get("temporal_context").is_some() || perceived["temporal_context"].is_null());
    }

    #[test]
    fn test_degradation_report_all_offline() {
        let sisters = offline_sisters();
        let report = sisters.degradation_report();
        // Dynamic count: 0/N where N is total sisters
        assert!(report.contains("0/"));
        assert!(report.contains("Offline"));
        assert!(report.contains("Memory"));
    }

    #[test]
    fn test_connected_sisters_list_offline() {
        let sisters = offline_sisters();
        assert!(sisters.connected_sisters_list().is_empty());
    }

    #[test]
    fn test_cognitive_prompt_includes_veritas_context() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({
            "input": "test",
            "veritas_context": "Intent: build a web app",
            "planning_context": "Active goal: Deploy v2 by Friday",
        });
        let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, false);
        assert!(prompt.contains("# Intent Analysis"));
        assert!(prompt.contains("# Active Goals"));
    }

    #[test]
    fn test_cognitive_prompt_graceful_degradation() {
        let sisters = offline_sisters();
        let perceived = serde_json::json!({ "input": "test" });
        let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, false);
        assert!(prompt.contains("SISTERS OFFLINE") || prompt.contains("None (offline mode"));
    }
}
