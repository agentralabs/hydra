//! System prompt builder — extracted from phase_think.rs for file size.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::cognitive::conversation_engine;
use crate::cognitive::decide::DecideEngine;
use crate::cognitive::inventions::InventionEngine;
use crate::sisters::SistersHandle;

use super::super::intent_router::ClassifiedIntent;
use super::llm_helpers::route_tools_for_prompt;
use super::phase_perceive::PerceiveResult;

/// Track the last memory hash to detect when we're returning the same facts.
static LAST_MEMORY_HASH: AtomicU64 = AtomicU64::new(0);

/// Build the COGNITIVE system prompt from perceived sister context.
pub(crate) fn build_system_prompt(
    text: &str,
    config: &super::super::loop_runner::CognitiveLoopConfig,
    intent: &ClassifiedIntent,
    perceive: &PerceiveResult,
    _is_simple: bool,
    is_complex: bool,
    is_action_request: bool,
    complexity: &str,
    sisters_handle: &Option<SistersHandle>,
    decide_engine: &Arc<DecideEngine>,
    inventions: &Option<Arc<InventionEngine>>,
    forge_blueprint: &Option<String>,
    veritas_intent: &Option<String>,
    _active_model: &str,
) -> String {
    if let Some(ref sh) = sisters_handle {
        let mut sp = sh.build_cognitive_prompt(&config.user_name, &perceive.perceived, is_complex);
        if let Some(ref blueprint) = forge_blueprint {
            sp.push_str(&format!("\n# Forge Blueprint (Pre-generated Architecture)\n{}\n\n", blueprint));
        }
        if let Some(ref intent_text) = veritas_intent {
            sp.push_str(&format!("\n# Compiled Intent\n{}\n\n", intent_text));
        }
        // Always-on memory injection with dedup detection + natural formatting
        if let Some(ref mem_context) = perceive.always_on_memory {
            let prev_hash = LAST_MEMORY_HASH.swap(perceive.memory_hash, Ordering::Relaxed);
            let is_duplicate = prev_hash != 0 && prev_hash == perceive.memory_hash;
            if is_duplicate {
                sp.push_str(
                    "\n# Memory Note\n\
                     The same memories were returned as the previous query. \
                     You don't have additional information beyond what you already shared. \
                     If the user asks again, acknowledge this honestly.\n\n",
                );
            } else {
                // Format memories naturally — never as bullet lists
                let mem_lines: Vec<String> = mem_context.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| l.to_string())
                    .collect();
                let formatted = conversation_engine::format_memories_naturally(&mem_lines);
                sp.push_str(&format!("\n# What You Remember\n{}\n\n", formatted));
            }
        }
        // Time + emotional context (conversation engine)
        let time_ctx = conversation_engine::build_time_context();
        sp.push_str(&format!(
            "\n# Current Context\nTime: {}, {}\n",
            time_ctx.time_of_day, time_ctx.day_of_week,
        ));
        if !time_ctx.contextual_note.is_empty() {
            sp.push_str(&format!("{}\n", time_ctx.contextual_note));
        }
        // Emotional read from user input — populate buffer with real history
        let mut conv_buf = conversation_engine::ConversationBuffer::new(15);
        for (role, content) in &config.history {
            conv_buf.add(role, content);
        }
        let emotional = conversation_engine::detect_emotional_context(text, &conv_buf);
        // User relationship depth
        let user_profile = conversation_engine::build_user_profile(
            &[], &config.user_name, config.session_count,
        );
        sp.push_str(&format!("\n# Relationship\n{}\n", user_profile));
        sp.push_str(&format!("\n# Read The Room\n{}\n\n", emotional));
        // Active belief injection with relevance filtering
        if let Some(ref beliefs) = perceive.beliefs_context {
            let input_words: Vec<String> = text.split_whitespace()
                .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                .filter(|w| w.len() >= 3)
                .collect();

            let semantic_map: &[(&[&str], &[&str])] = &[
                (&["database", "db", "sql"], &["postgres", "mysql", "sqlite", "mongo", "redis", "supabase", "dynamo"]),
                (&["framework", "frontend", "ui"], &["react", "vue", "angular", "svelte", "next", "nuxt"]),
                (&["backend", "server", "api"], &["express", "fastapi", "django", "flask", "actix", "axum", "rails"]),
                (&["language", "lang", "programming"], &["rust", "python", "javascript", "typescript", "go", "java", "kotlin"]),
                (&["testing", "test"], &["jest", "pytest", "vitest", "mocha", "cargo test"]),
                (&["deploy", "hosting", "cloud"], &["aws", "gcp", "azure", "vercel", "netlify", "fly.io", "docker"]),
                (&["package", "dependency"], &["npm", "yarn", "pip", "cargo", "pnpm", "bun"]),
            ];
            let expanded_words: Vec<String> = {
                let mut expanded = input_words.clone();
                for (triggers, related) in semantic_map {
                    if input_words.iter().any(|w| triggers.contains(&w.as_str())) {
                        for r in *related {
                            expanded.push(r.to_string());
                        }
                    }
                }
                expanded
            };

            let relevant_lines: Vec<&str> = beliefs.lines()
                .filter(|line| {
                    let lower = line.to_lowercase();
                    expanded_words.iter().any(|w| lower.contains(w))
                    || lower.contains("[correction]")
                    || lower.contains("[convention]")
                })
                .collect();

            if !relevant_lines.is_empty() {
                sp.push_str(&format!(
                    "\n# Known Constraints & Preferences\n\
                     These beliefs are confirmed from past interactions. Respect them:\n{}\n\n",
                    relevant_lines.join("\n")
                ));
            } else if !beliefs.is_empty() {
                sp.push_str(&format!("\n# Active Beliefs\nKnown facts and preferences:\n{}\n\n", beliefs));
            }
        }
        // Temporal memory injection
        if let Some(ref inv) = inventions {
            if let Some(temporal_ctx) = inv.recall_temporal_context(text, 5) {
                sp.push_str(&format!("\n# Recent Context\n{}\n\n", temporal_ctx));
            }
            // Failure context injection
            if let Some(failure_ctx) = inv.recall_temporal_context("FAILED", 3) {
                let input_lower = text.to_lowercase();
                let input_words: Vec<&str> = input_lower.split_whitespace()
                    .filter(|w| w.len() >= 3)
                    .collect();
                let relevant_failures: Vec<&str> = failure_ctx.lines()
                    .filter(|line| {
                        let lower = line.to_lowercase();
                        input_words.iter().any(|w| lower.contains(w))
                    })
                    .collect();
                if !relevant_failures.is_empty() {
                    sp.push_str(&format!(
                        "\n# Previous Failures on This Topic\n\
                         These related attempts failed before. Avoid repeating the same mistakes:\n{}\n\n",
                        relevant_failures.join("\n")
                    ));
                }
            }
        }
        // Trust level injection
        let trust = decide_engine.current_trust();
        let autonomy = decide_engine.current_level();
        sp.push_str(&format!(
            "\n# Trust & Autonomy\nCurrent trust score: {:.0}%\nAutonomy level: {:?}\nThis reflects how much the user trusts Hydra based on interaction history.\n\n",
            trust * 100.0, autonomy,
        ));
        // Tool routing
        let routed_tools = route_tools_for_prompt(intent, complexity, is_action_request, sh, text);
        let tool_line_count = routed_tools.lines().count();
        eprintln!("[hydra:tools] Routed {} tool lines for intent={:?} complexity={}", tool_line_count, intent.category, complexity);
        if !routed_tools.is_empty() {
            sp.push_str(&format!("\n# Available Tools\n{}\n\n", routed_tools));
        }
        // Federation status
        if let Some(ref fed_ctx) = perceive.federation_context {
            sp.push_str(&format!("\n# Federation Status\n{}\n\n", fed_ctx));
        }
        sp
    } else {
        format!(
            "You are Hydra, a cognitive AI orchestrator built by Agentra Labs. \
             You are NOT a chatbot — you are an agentic executor that DOES things. \
             You can execute commands, create projects, access APIs, deploy to cloud, \
             federate across systems, and integrate with any service the user provides credentials for. \
             {}When the user asks you to do something, DO IT — never say \"I can't\" for things you can do. \
             If you need credentials or access, ask for them specifically.\n\n\
             ## How to Execute Commands:\n\
             When the user asks you to DO something (open an app, run a command, check something, read a file, \
             browse the web, access a directory), you MUST wrap the shell command in <hydra-exec> tags. \
             This is how you actually execute actions on the user's machine.\n\n\
             Examples:\n\
             - User: \"what's in this folder?\" → <hydra-exec>ls -la</hydra-exec>\n\
             - User: \"read this file\" → <hydra-exec>cat ~/path/to/file.md</hydra-exec>\n\
             - User: \"open my terminal\" → <hydra-exec>open -a Terminal</hydra-exec>\n\
             - User: \"browse the internet for top stories\" → <hydra-exec>curl -s 'https://hacker-news.firebaseio.com/v0/topstories.json?print=pretty' | head -20</hydra-exec>\n\
             - User: \"access this directory\" → <hydra-exec>ls -la ~/Documents/path</hydra-exec>\n\n\
             CRITICAL: Without <hydra-exec> tags, you are ONLY talking. The tags make things HAPPEN. \
             Always use them when the user wants you to DO something, not just talk about it. \
             Never say \"Let me do X\" without including the actual <hydra-exec> command. \
             You can include multiple <hydra-exec> tags in one response. Each will be executed in order.\n\
             The command output will be captured and shown to the user.",
            if config.user_name.is_empty() { String::new() } else { format!("The user's name is {}. ", config.user_name) }
        )
    }
}
