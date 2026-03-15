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

        "/memory" => {
            match args {
                "all" => Some("__TEXT__:__MEMORY_MODE__:all:Memory capture set to: **Full Conversation** -- every exchange stored.".to_string()),
                "facts" => Some("__TEXT__:__MEMORY_MODE__:facts:Memory capture set to: **Facts Only** -- decisions and preferences stored.".to_string()),
                "none" | "off" => Some("__TEXT__:__MEMORY_MODE__:none:Memory capture set to: **Disabled** -- nothing stored.".to_string()),
                "" => Some("__TEXT__:**Memory Capture**\n\nUsage: `/memory all` | `/memory facts` | `/memory none`\n\n- **all**: Full conversation capture (every exchange)\n- **facts**: Facts only (decisions, preferences, corrections)\n- **none**: Disabled (session-only, nothing persisted)".to_string()),
                _ => Some(format!("__TEXT__:Unknown mode '{}'. Use: `/memory all`, `/memory facts`, or `/memory none`", args)),
            }
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

        // ── Threat Intelligence (P11) ──
        "/threat" => {
            let correlator = crate::threat::ThreatCorrelator::new();
            let content = match args {
                "history" => correlator.signal_history(20),
                "patterns" => correlator.patterns_summary(),
                _ => correlator.summary(),
            };
            Some(format!("__TEXT__:{}", content))
        }

        // P10: Sister improve — async, falls through to handler
        "/improve-sister" if args.is_empty() => {
            Some("__TEXT__:Usage: `/improve-sister <path> <goal>`\nExamples: `/improve-sister ../agentic-memory add retry logic`".to_string())
        }
        // P9/P10: Handled by TUI app state or cognitive loop dispatch
        "/improve-sister" | "/swarm" | "/swarm-status" | "/swarm-spawn" | "/swarm-assign"
        | "/swarm-results" | "/swarm-kill" | "/swarm-kill-all" | "/swarm-scale" => None,

        _ => None,
    }
}

/// Select which MCP tools to include in the LLM prompt based on intent.
/// Delegates to llm_tool_routing.rs for the full routing table across all 17 sisters.
pub(crate) fn route_tools_for_prompt(
    intent: &crate::cognitive::intent_router::ClassifiedIntent,
    complexity: &str,
    is_action: bool,
    sisters: &crate::sisters::cognitive::Sisters,
    user_text: &str,
) -> String {
    // Direct-handled intents don't need LLM tools
    if intent.category.has_direct_handler() && intent.confidence >= 0.6 {
        return String::new();
    }
    let tools = super::llm_tool_routing::route_tools_for_intent(
        intent, sisters, user_text, complexity, is_action,
    );
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
    out.push_str("You can call these MCP tools using <hydra-tool name=\"tool_name\">{\"param\": \"value\"}</hydra-tool> tags:\n");
    for (prefix, names) in &by_prefix {
        out.push_str(&format!("- {}: {}\n", prefix, names.join(", ")));
    }
    out.push_str(&format!("({} tools available)\n", tools.len()));
    out.push_str("Tool results will be returned to you. When your task is complete, include <hydra-done/> at the end.\n");
    out
}
