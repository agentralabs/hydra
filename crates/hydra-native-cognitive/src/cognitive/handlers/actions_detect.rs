//! Direct action detection — identifies user intent and returns shell commands.
//! Split from actions.rs for compilation performance.

use super::platform::*;
use super::actions::{
    extract_browser_name, extract_url_intent, extract_dependency_target,
    extract_path_from_text, shell_escape, extract_app_name_from_intent,
};

/// PRE-LLM OPTIMIZATION: Detects simple direct actions before spending tokens on LLM.
/// Catches: "create file X", "run tests", "open X", "delete X" etc.
/// Falls through to LLM for anything not matched. Saves ~150 tokens per simple action.
/// Known limitation: English-only patterns.
///
/// Universal action executor — detects user intent and returns the appropriate shell command.
/// Works across macOS, Linux, and Windows. No hardcoded app list — resolves ANY app by name.
pub(crate) fn detect_direct_action_command(text: &str) -> Option<String> {
    let lower = text.to_lowercase();

    // ── Special case: Terminal (needs new window, not just focus) ──
    if (lower.contains("open") && lower.contains("terminal"))
        || lower.contains("new terminal")
        || lower.contains("fresh terminal")
        || (lower.contains("continue") && lower.contains("terminal"))
    {
        return Some(platform_new_terminal());
    }

    // ── Special case: New browser tab ──
    if lower.contains("new tab") || (lower.contains("open") && lower.contains("tab")) {
        let browser = extract_browser_name(&lower);
        return Some(platform_new_tab(&browser));
    }

    // ── URL detection: "open google.com" / "open https://..." / "go to example.com" ──
    if let Some(url) = extract_url_intent(&lower, text) {
        return Some(platform_open_url(&url));
    }

    // ── Top stories / latest news / headlines → fetch via HackerNews API ──
    if lower.contains("top stories") || lower.contains("latest news")
        || lower.contains("headlines") || lower.contains("trending news")
        || lower.contains("what's happening") || lower.contains("news today")
    {
        return Some(
            "echo '=== Top Stories ===' && \
             curl -s 'https://hacker-news.firebaseio.com/v0/topstories.json' | \
             python3 -c \"import sys,json; ids=json.load(sys.stdin)[:10]; \
             [print(json.loads(__import__('urllib.request').urlopen(\
             f'https://hacker-news.firebaseio.com/v0/item/{i}.json').read())['title']) \
             for i in ids]\" 2>/dev/null || echo 'Could not fetch stories — try: open https://news.ycombinator.com'"
            .to_string()
        );
    }

    // ── Codebase search — smart grep excluding binaries/target ──
    let is_codebase_search = lower.contains("codebase") || lower.contains("code base")
        || lower.contains("project") || lower.contains("repository") || lower.contains("repo")
        || lower.contains("source code") || lower.contains("in the code")
        || lower.contains("in my code") || lower.contains("in our code")
        || lower.contains("in the files") || lower.contains("in my files");
    if is_codebase_search && (lower.contains("search") || lower.contains("find") || lower.contains("look for") || lower.contains("grep")) {
        // Extract the search term
        let query = if let Some(pos) = lower.find("for ") {
            text[pos + 4..].trim()
        } else if let Some(pos) = lower.find("search ") {
            text[pos + 7..].trim()
        } else {
            text.trim()
        };
        // Strip trailing context like "in the codebase", "in the project"
        let query = query
            .trim_end_matches(" in the codebase").trim_end_matches(" in the code base")
            .trim_end_matches(" in the project").trim_end_matches(" in the repository")
            .trim_end_matches(" in the repo").trim_end_matches(" in the code")
            .trim_end_matches(" in my code").trim_end_matches(" in our code")
            .trim_end_matches(" in the files").trim_end_matches(" in my files")
            .trim_end_matches(" in the source code")
            .trim();
        if !query.is_empty() {
            let escaped = query.replace('\'', "'\\''");
            return Some(format!(
                "grep -rn --include='*.rs' --include='*.ts' --include='*.tsx' --include='*.js' \
                 --include='*.py' --include='*.toml' --include='*.json' --include='*.md' \
                 --include='*.css' --include='*.html' --include='*.yaml' --include='*.yml' \
                 --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.git \
                 --exclude-dir=out --exclude-dir=dist --exclude-dir=build \
                 -i '{}' . | head -50",
                escaped
            ));
        }
    }

    // ── Dependency / usage queries — "what depends on X", "who calls X", "where is X used" ──
    let is_dependency_query = lower.contains("depends on") || lower.contains("what depends")
        || lower.contains("who uses") || lower.contains("what imports")
        || lower.contains("impact of") || lower.contains("what calls")
        || lower.contains("who calls") || lower.contains("references to")
        || lower.contains("where is") && lower.contains("used")
        || lower.contains("what uses") || lower.contains("who imports");
    if is_dependency_query {
        // Extract the search target — look for file names, function names, identifiers
        let target = extract_dependency_target(text, &lower);
        if let Some(term) = target {
            let escaped = term.replace('\'', "'\\''");
            // For file-based queries, search for the module name (without extension)
            let search_term = escaped.trim_end_matches(".rs").trim_end_matches(".ts")
                .trim_end_matches(".py").trim_end_matches(".js");
            return Some(format!(
                "echo '=== Files referencing \"{}\" ===' && \
                 grep -rn --include='*.rs' --include='*.ts' --include='*.tsx' --include='*.js' \
                 --include='*.py' --include='*.toml' --include='*.json' --include='*.md' \
                 --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.git \
                 --exclude-dir=out --exclude-dir=dist --exclude-dir=build \
                 -i '{}' . | head -50",
                escaped, search_term
            ));
        }
    }

    // ── "Browse the internet for X" / "search for X" → open a web search ──
    if !is_codebase_search && (
        (lower.contains("browse") && lower.contains("internet"))
        || lower.starts_with("search for ")
        || lower.starts_with("google ")
        || lower.starts_with("look up ")
    ) {
        // Extract the search query
        let query = if let Some(pos) = lower.find("for ") {
            &text[pos + 4..]
        } else if lower.starts_with("google ") {
            &text[7..]
        } else if lower.starts_with("look up ") {
            &text[8..]
        } else {
            text
        };
        let encoded = query.trim().replace(' ', "+");
        return Some(platform_open_url(&format!("https://www.google.com/search?q={}", encoded)));
    }

    // ── Scroll / navigate within an app ──
    if lower.contains("scroll") {
        let direction = if lower.contains("down") { "down" } else if lower.contains("up") { "up" } else { "down" };
        let amount = if lower.contains("bottom") || lower.contains("end") { "max" } else { "page" };
        return Some(platform_scroll(direction, amount));
    }

    // ── Type / input text into focused app ──
    if lower.starts_with("type ") || lower.starts_with("enter ") {
        let content = if lower.starts_with("type ") { &text[5..] } else { &text[6..] };
        return Some(platform_type_text(content.trim()));
    }

    // ── Screenshot ──
    if lower.contains("screenshot") || lower.contains("screen capture") || lower.contains("screen shot") {
        return Some(platform_screenshot());
    }

    // ── System info ──
    if lower.contains("system info") || lower.contains("system information")
        || lower.contains("what os") || lower.contains("what system")
    {
        return Some(platform_system_info());
    }

    // ── Kill / close / quit an app ──
    if (lower.contains("close") || lower.contains("quit") || lower.contains("kill"))
        && !lower.contains("close the door") && !lower.contains("kill the")
    {
        if let Some(app) = extract_app_name_from_intent(&lower, &["close", "quit", "kill"]) {
            return Some(platform_close_app(&app));
        }
    }

    // ── Minimize / hide ──
    if lower.contains("minimize") || lower.contains("hide") {
        if let Some(app) = extract_app_name_from_intent(&lower, &["minimize", "hide"]) {
            return Some(platform_minimize_app(&app));
        }
    }

    // ── Read file / access directory — direct filesystem commands ──
    if (lower.contains("read") || lower.contains("show me") || lower.contains("cat "))
        && !lower.contains("read my mind")
    {
        // Extract path-like tokens from the original text
        if let Some(path) = extract_path_from_text(text) {
            return Some(format!("cat {}", shell_escape(&path)));
        }
    }
    if (lower.contains("access") || lower.contains("what's in") || lower.contains("whats in")
        || lower.contains("list"))
        && !lower.contains("access denied") && !lower.contains("access control")
    {
        if let Some(path) = extract_path_from_text(text) {
            return Some(format!("ls -la {}", shell_escape(&path)));
        }
        // "what's in this folder" / "what's in here" / "list this directory" → current dir
        if lower.contains("this folder") || lower.contains("this directory")
            || lower.contains("this dir") || lower.contains("in here")
            || lower.contains("current folder") || lower.contains("current directory")
            || (lower.contains("what") && lower.contains("in") && !lower.contains("in my"))
        {
            return Some("ls -la .".to_string());
        }
    }

    // ── Universal "open X" — resolves ANY app by name ──
    // This MUST be last since it's the most generic matcher
    if lower.starts_with("open ") || lower.starts_with("launch ") || lower.starts_with("start ") {
        let verb_len = if lower.starts_with("launch ") { 7 } else if lower.starts_with("start ") { 6 } else { 5 };
        let raw_target = text[verb_len..].trim();
        // Strip articles: "open the calculator" → "calculator"
        let target = strip_articles(raw_target);

        if !target.is_empty() {
            return Some(platform_open_app(&target));
        }
    }

    None
}
