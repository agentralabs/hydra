//! LLM command utilities — slash commands, project detection, tool routing, topic extraction.

/// Phase 2, L2: Extract the primary topic from user input for hot-topic tracking.
pub(crate) fn extract_primary_topic(input: &str) -> String {
    let stop_words = ["the", "a", "an", "is", "are", "was", "were", "be", "been",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "can", "may", "might", "shall", "must", "to", "of", "in",
        "for", "on", "with", "at", "by", "from", "as", "into", "about",
        "my", "your", "our", "his", "her", "its", "what", "how", "why",
        "when", "where", "which", "who", "whom", "that", "this", "these",
        "those", "i", "you", "we", "they", "he", "she", "it", "me",
        "him", "us", "them", "and", "but", "or", "not", "no", "so",
        "if", "just", "also", "very", "too", "please", "help", "need",
        "want", "like", "know", "think", "tell", "show", "make", "get"];

    let words: Vec<&str> = input.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() >= 3 && !stop_words.contains(&w.to_lowercase().as_str()))
        .take(3)
        .collect();

    words.join("_").to_lowercase()
}

/// Detect project type and return the appropriate shell command.
/// Uses shell conditionals so it works regardless of cwd at compile time.
pub(crate) fn detect_project_command(action: &str, extra_args: &str) -> String {
    // Use shell conditionals to detect project type at runtime
    match action {
        "test" => format!("if [ -f Cargo.toml ]; then cargo test{}; elif [ -f package.json ]; then npm test; elif [ -f pyproject.toml ]; then python -m pytest; elif [ -f go.mod ]; then go test ./...; else echo 'No project detected'; fi", extra_args),
        "build" => format!("if [ -f Cargo.toml ]; then cargo build{}; elif [ -f package.json ]; then npm run build; elif [ -f go.mod ]; then go build ./...; else echo 'No project detected'; fi", extra_args),
        "run" => format!("if [ -f Cargo.toml ]; then cargo run{}; elif [ -f package.json ]; then npm start; elif [ -f go.mod ]; then go run .; else echo 'No project detected'; fi", extra_args),
        "lint" => "if [ -f Cargo.toml ]; then cargo clippy 2>&1; elif [ -f package.json ]; then npx eslint .; elif [ -f pyproject.toml ]; then python -m ruff check .; else echo 'No project detected'; fi".to_string(),
        "fmt" => "if [ -f Cargo.toml ]; then cargo fmt; elif [ -f package.json ]; then npx prettier --write .; elif [ -f pyproject.toml ]; then python -m black .; else echo 'No project detected'; fi".to_string(),
        "bench" => "if [ -f Cargo.toml ]; then cargo bench; elif [ -f package.json ]; then npm run bench 2>/dev/null || echo 'No bench script'; else echo 'No project detected'; fi".to_string(),
        "doc" => "if [ -f Cargo.toml ]; then cargo doc --open; elif [ -f package.json ]; then npm run docs 2>/dev/null || echo 'No docs script'; else echo 'No project detected'; fi".to_string(),
        "deps" => "if [ -f Cargo.toml ]; then cargo tree --depth 1; elif [ -f package.json ]; then cat package.json | python3 -c \"import sys,json; d=json.load(sys.stdin); [print(f'{k}: {v}') for k,v in {**d.get('dependencies',{}),**d.get('devDependencies',{})}.items()]\"; else echo 'No project detected'; fi".to_string(),
        _ => format!("echo 'Unknown action: {}'", action),
    }
}

/// Universal slash command handler — works in both Desktop and TUI.
/// Returns the shell command to execute, or "__TEXT__:content" for
/// static text responses. Returns None if unrecognized.
pub(crate) fn handle_universal_slash_command(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let (cmd, args) = match trimmed.find(' ') {
        Some(pos) => (&trimmed[..pos], trimmed[pos + 1..].trim()),
        None => (trimmed, ""),
    };

    match cmd {
        // ── Developer commands ──
        "/test" => {
            let extra = if args.is_empty() { String::new() } else { format!(" {}", args) };
            Some(detect_project_command("test", &extra))
        }
        "/build" => {
            let extra = if args.is_empty() { String::new() } else { format!(" {}", args) };
            Some(detect_project_command("build", &extra))
        }
        "/run" => {
            let extra = if args.is_empty() { String::new() } else { format!(" {}", args) };
            Some(detect_project_command("run", &extra))
        }
        "/lint" => Some(detect_project_command("lint", "")),
        "/fmt" => Some(detect_project_command("fmt", "")),
        "/bench" => Some(detect_project_command("bench", "")),
        "/doc" => Some(detect_project_command("doc", "")),
        "/deps" => Some(detect_project_command("deps", "")),

        "/files" => {
            // Show project tree (depth 3, max 200 entries)
            Some("find . -maxdepth 3 -not -path '*/target/*' -not -path '*/.git/*' -not -path '*/node_modules/*' -not -path '*/.next/*' | head -200 | sort".to_string())
        }

        "/git" => {
            if args.is_empty() || args == "status" {
                Some("git status && echo '---' && git log --oneline -5".to_string())
            } else if args.starts_with("log") {
                let n = args.strip_prefix("log").unwrap_or("").trim();
                let count = n.parse::<u32>().unwrap_or(10);
                Some(format!("git log --oneline -{}", count))
            } else if args.starts_with("diff") {
                Some(format!("git {}", args))
            } else if args.starts_with("branch") {
                Some("git branch -a".to_string())
            } else {
                Some(format!("git {}", args))
            }
        }

        "/search" => {
            if args.is_empty() {
                Some("__TEXT__:Usage: `/search <pattern>` — searches code for a pattern".to_string())
            } else {
                Some(format!(
                    "grep -rn --include='*.rs' --include='*.ts' --include='*.tsx' --include='*.js' \
                     --include='*.py' --include='*.go' --include='*.toml' --include='*.json' \
                     '{}' . 2>/dev/null | head -50",
                    args.replace('\'', "'\\''")
                ))
            }
        }

        "/symbols" => {
            if args.is_empty() {
                Some("__TEXT__:Usage: `/symbols <file>` — extracts functions and types from a file".to_string())
            } else {
                // Rust-aware symbol extraction
                Some(format!(
                    "grep -n '^\\s*\\(pub\\s\\+\\)\\?\\(fn\\|struct\\|enum\\|trait\\|impl\\|type\\|mod\\|const\\|static\\)\\s' {} 2>/dev/null || \
                     grep -n '\\(function\\|class\\|interface\\|type\\|export\\)' {} 2>/dev/null || \
                     grep -n '\\(def\\|class\\)' {} 2>/dev/null || \
                     echo 'No symbols found in {}'",
                    args, args, args, args
                ))
            }
        }

        // ── System commands ──
        "/sisters" | "/status" => {
            // This will be handled by the sister diagnostic path in the cognitive loop
            // Return None to let it fall through to the normal path
            None
        }

        "/env" => {
            let profile = crate::environment::EnvironmentProfile::probe_all();
            let output = if args == "refresh" {
                format!("Re-probed environment.\n\n{}", profile.display_full())
            } else {
                profile.display_full()
            };
            Some(format!("__TEXT__:{}", output))
        }

        "/health" => {
            Some("echo '=== System Health ===' && uptime && echo '---' && df -h . && echo '---' && free -h 2>/dev/null || vm_stat 2>/dev/null".to_string())
        }

        "/clear" | "/compact" | "/history" => {
            // UI-only commands — can't handle here, return text hint
            Some("__TEXT__:This command is handled by the UI layer. Use the Desktop or TUI interface directly.".to_string())
        }

        "/model" => {
            if args.is_empty() {
                Some("__TEXT__:Current model is set in Settings. Use `/model <name>` to change.".to_string())
            } else {
                Some(format!("__TEXT__:Model preference noted: **{}**. Change it in Settings to apply.", args))
            }
        }

        "/version" => {
            let version = env!("CARGO_PKG_VERSION");
            Some(format!(
                "__TEXT__:## Hydra v{}\n\n\
                 **Engine:** hydra-native-cognitive\n\
                 **Runtime:** Rust (native)\n\n\
                 Sister count and autonomy level are shown in `/status`.",
                version
            ))
        }

        "/test-repo" => {
            if args.is_empty() {
                Some("__TEXT__:Usage: `/test-repo <url>` — clone, understand, setup, test, and report on a repository.\n\
                    Options: `/test-repo <url> --dry-run` — show plan without executing.".to_string())
            } else {
                let dry_run = args.contains("--dry-run");
                let url_part = args.replace("--dry-run", "").trim().to_string();
                if let Some(url) = crate::project_exec::extract_url(&url_part)
                    .or_else(|| if url_part.contains('.') { Some(url_part.clone()) } else { None })
                {
                    let mode = if dry_run { "DRY RUN" } else { "EXECUTE" };
                    Some(format!(
                        "__TEXT__:__PROJECT_EXEC__:{}:{}",
                        mode, url
                    ))
                } else {
                    Some("__TEXT__:Invalid URL. Usage: `/test-repo <url>`".to_string())
                }
            }
        }

        "/help" => {
            Some("__TEXT__:## Slash Commands\n\n\
                **Developer:** /test, /build, /run, /files, /git, /search, /symbols, /lint, /fmt, /deps, /bench, /doc\n\
                **System:** /sisters, /health, /status, /version, /env\n\
                **Autonomy:** /test-repo\n\
                **Conversation:** /clear, /compact, /history\n\
                **Settings:** /model, /theme, /voice\n\
                **Control:** /approve, /deny, /kill\n\
                **Debug:** /help, /tokens, /log\n\n\
                Type `/` to see autocomplete suggestions.".to_string())
        }

        _ => None,
    }
}

/// Select which MCP tools to include in the LLM prompt based on intent.
/// Returns a formatted string of tool names grouped by sister, or empty string
/// if no tools are needed (Tier 0).
pub(crate) fn route_tools_for_prompt(
    intent: &crate::cognitive::intent_router::ClassifiedIntent,
    complexity: &str,
    is_action: bool,
    sisters: &crate::sisters::cognitive::Sisters,
    user_text: &str,
) -> String {
    use crate::cognitive::intent_router::IntentCategory;

    // Direct-handled intents don't need LLM tools
    if intent.category.has_direct_handler() && intent.confidence >= 0.6 {
        return String::new();
    }

    let mut tools: Vec<String> = Vec::new();

    match intent.category {
        // Memory recall → memory + cognition tools
        IntentCategory::MemoryRecall => {
            tools.extend(sisters.tools_for_sister("memory", &[
                "memory_query", "memory_similar", "memory_temporal",
                "memory_context", "memory_search",
            ]));
            tools.extend(sisters.tools_for_sister("cognition", &[
                "cognition_belief_query", "cognition_belief_list",
            ]));
        }
        // Code tasks → forge + codebase + memory
        IntentCategory::CodeBuild | IntentCategory::CodeFix | IntentCategory::CodeExplain => {
            tools.extend(sisters.tools_for_sister("forge", &[
                "forge_blueprint", "forge_skeleton", "forge_structure",
            ]));
            tools.extend(sisters.tools_for_sister("codebase", &[
                "symbol_lookup", "impact_analysis", "graph_stats",
                "search_semantic", "search_code",
            ]));
            tools.extend(sisters.tools_for_sister("memory", &[
                "memory_query", "memory_context",
            ]));
        }
        // Planning → planning + time + memory
        IntentCategory::PlanningQuery => {
            tools.extend(sisters.tools_for_sister("planning", &[
                "planning_goal", "planning_progress", "planning_decision",
            ]));
            tools.extend(sisters.tools_for_sister("time", &[
                "time_deadline_check", "time_deadline_add", "time_schedule_query",
            ]));
            tools.extend(sisters.tools_for_sister("memory", &[
                "memory_query", "memory_temporal",
            ]));
        }
        // Web/browse → vision tools
        IntentCategory::WebBrowse => {
            tools.extend(sisters.tools_for_sister("vision", &[
                "vision_capture", "vision_query", "vision_ocr",
                "vision_compare", "vision_ground",
            ]));
        }
        // Communication → comm tools
        IntentCategory::Communicate => {
            tools.extend(sisters.tools_for_sister("comm", &[
                "comm_message", "comm_channel", "comm_federation",
                "comm_send", "comm_notify",
            ]));
        }
        // Unknown/Question → route by complexity, with smart detection
        IntentCategory::Unknown | IntentCategory::Question => {
            let lower_input = user_text.to_lowercase();

            // Even simple queries need tools if they mention specific sisters/capabilities
            let needs_identity = lower_input.contains("receipt") || lower_input.contains("prove")
                || lower_input.contains("trust") || lower_input.contains("what did you")
                || lower_input.contains("what have you") || lower_input.contains("last action");
            let needs_time = lower_input.contains("deadline") || lower_input.contains("schedule")
                || lower_input.contains("when") || lower_input.contains("how long");
            let needs_planning = lower_input.contains("goal") || lower_input.contains("plan")
                || lower_input.contains("what should") || lower_input.contains("next step");

            if needs_identity {
                tools.extend(sisters.tools_for_sister("identity", &[
                    "identity_show", "receipt_list",
                ]));
            }
            if needs_time {
                tools.extend(sisters.tools_for_sister("time", &[
                    "time_schedule", "time_deadline", "time_deadline_check",
                ]));
            }
            if needs_planning {
                tools.extend(sisters.tools_for_sister("planning", &[
                    "planning_goal", "planning_progress",
                ]));
            }

            if complexity == "complex" || is_action {
                // Broad tool set for complex unknown intents
                tools.extend(sisters.tools_for_sister("memory", &[
                    "memory_query", "memory_context", "memory_similar",
                ]));
                tools.extend(sisters.tools_for_sister("codebase", &[
                    "symbol_lookup", "impact_analysis", "search_semantic",
                ]));
                tools.extend(sisters.tools_for_sister("forge", &[
                    "forge_blueprint", "forge_skeleton",
                ]));
                tools.extend(sisters.tools_for_sister("vision", &[
                    "vision_capture", "vision_query",
                ]));
                if !needs_identity {
                    tools.extend(sisters.tools_for_sister("identity", &[
                        "identity_show", "receipt_list",
                    ]));
                }
                if !needs_planning {
                    tools.extend(sisters.tools_for_sister("planning", &[
                        "planning_goal", "planning_progress",
                    ]));
                }
                tools.extend(sisters.tools_for_sister("cognition", &[
                    "cognition_model", "cognition_predict",
                ]));
                tools.extend(sisters.tools_for_sister("reality", &[
                    "reality_deployment", "reality_environment",
                ]));
                tools.extend(sisters.tools_for_sister("veritas", &[
                    "veritas_compile", "veritas_verify",
                ]));
                tools.extend(sisters.tools_for_sister("aegis", &[
                    "shadow_simulate", "aegis_validate",
                ]));
                tools.extend(sisters.tools_for_sister("comm", &[
                    "comm_send", "comm_message",
                ]));
                if !needs_time {
                    tools.extend(sisters.tools_for_sister("time", &[
                        "time_schedule", "time_deadline",
                    ]));
                }
                tools.truncate(30);
            }
        }
        // All other categories are direct-handled (no LLM tools needed)
        _ => {}
    }

    format_tool_list(&tools)
}

/// Format a list of tool names into a concise prompt section.
pub(crate) fn format_tool_list(tools: &[String]) -> String {
    if tools.is_empty() {
        return String::new();
    }
    let mut by_prefix: std::collections::BTreeMap<String, Vec<&str>> = std::collections::BTreeMap::new();
    for tool in tools {
        let prefix = tool.split('_').next().unwrap_or("other").to_string();
        by_prefix.entry(prefix).or_default().push(tool);
    }
    let mut out = String::new();
    out.push_str("You can call these MCP tools using <hydra-tool> tags:\n");
    for (prefix, names) in &by_prefix {
        out.push_str(&format!("- {}: {}\n", prefix, names.join(", ")));
    }
    out.push_str(&format!("({} tools available)\n", tools.len()));
    out
}
