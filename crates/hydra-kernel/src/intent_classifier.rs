//! Intent classifier — LLM micro-call to classify user input into agent intents.
//!
//! Replaces hardcoded keyword matching with cheap LLM classification.
//! Uses the cheapest available model (~50 tokens per call).

use std::collections::HashMap;

/// Classified intent for routing to the correct agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentIntent {
    /// Multi-step browser task (navigate, fill forms, interact with pages).
    BrowserAgent,
    /// Simple URL fetch (just get the page content).
    BrowserFetch,
    /// Desktop GUI interaction (non-browser application control).
    Desktop,
    /// Shell command execution.
    Shell,
    /// File system operations (read, write, search).
    File,
    /// Web search for information.
    Search,
    /// General task execution — user wants something DONE (create, build, deploy, etc.).
    /// Routes to the conductor for step decomposition and execution.
    Action,
    /// Normal conversation — no agent needed.
    Conversation,
}

impl AgentIntent {
    /// Whether this intent should be routed to the conductor for execution.
    pub fn is_actionable(&self) -> bool {
        matches!(self, Self::Action | Self::Shell | Self::File)
    }
}

impl std::fmt::Display for AgentIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AgentIntent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BrowserAgent => "browser_agent",
            Self::BrowserFetch => "browser_fetch",
            Self::Desktop => "desktop",
            Self::Shell => "shell",
            Self::File => "file",
            Self::Search => "search",
            Self::Action => "action",
            Self::Conversation => "conversation",
        }
    }

    fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "browser_agent" | "browser agent" => Self::BrowserAgent,
            "browser_fetch" | "browser fetch" | "browser" => Self::BrowserFetch,
            "desktop" => Self::Desktop,
            "shell" => Self::Shell,
            "file" | "filesystem" => Self::File,
            "search" | "web_search" => Self::Search,
            "action" | "task" | "execute" => Self::Action,
            _ => Self::Conversation,
        }
    }
}

/// Classify user input into an agent intent using an LLM micro-call.
/// Falls back to heuristic if LLM is unavailable.
pub async fn classify(
    input: &str,
    api_key: Option<&str>,
) -> AgentIntent {
    // Try LLM classification first
    if let Some(key) = api_key {
        if let Some(intent) = classify_via_llm(input, key).await {
            return intent;
        }
    }

    // Fallback: basic heuristic (only for when LLM is unavailable)
    classify_heuristic(input)
}

/// Inject intent classification results into enrichments.
pub fn inject_enrichments(
    intent: &AgentIntent,
    enrichments: &mut HashMap<String, String>,
) {
    enrichments.insert("agent_intent".into(), intent.as_str().into());
    match intent {
        AgentIntent::BrowserAgent | AgentIntent::BrowserFetch => {
            enrichments
                .entry("browser_relevant".into())
                .or_insert_with(|| "true".into());
        }
        AgentIntent::Action | AgentIntent::Shell | AgentIntent::File => {
            enrichments.insert("actionable".into(), "true".into());
        }
        _ => {}
    }
}

async fn classify_via_llm(input: &str, _api_key: &str) -> Option<AgentIntent> {
    let prompt = format!(
        "Classify this user input into exactly ONE category. Reply with ONLY the category name.\n\
         Categories:\n\
         - action: user wants something DONE (create, build, deploy, install, delete, run a task)\n\
         - browser_agent: multi-step browser interaction (fill forms, login, post content)\n\
         - browser_fetch: just visit a URL and get content\n\
         - desktop: control a desktop GUI app (not browser)\n\
         - shell: execute a specific shell command\n\
         - file: file system operations (read, write, search files)\n\
         - search: web search for information\n\
         - conversation: question, discussion, explanation — no action needed\n\
         Input: {input}"
    );
    let content = crate::loop_::llm::LlmCaller::micro_call(&prompt).await?;
    eprintln!("hydra-kernel: intent classified as: {content}");
    Some(AgentIntent::from_str(&content))
}

/// Synchronous classification for use in middleware (non-async context).
/// Uses heuristic only — the async `classify()` is preferred when possible.
pub fn classify_heuristic_sync(input: &str, _api_key: Option<&str>) -> AgentIntent {
    classify_heuristic(input)
}

/// Basic heuristic fallback when LLM is unavailable.
/// This is intentionally simple — the LLM path is preferred.
fn classify_heuristic(input: &str) -> AgentIntent {
    let lower = input.to_lowercase();

    // "open my browser/chrome/firefox/safari" → Desktop (visible app, not headless)
    if lower.contains("open") && (lower.contains("browser") || lower.contains("chrome")
        || lower.contains("firefox") || lower.contains("safari")) {
        return AgentIntent::Desktop;
    }
    // "open TextEdit/Finder/etc" → Desktop
    if lower.starts_with("open ") && !lower.contains("http") && !lower.contains(".com") {
        return AgentIntent::Desktop;
    }

    let has_url = input.contains("http://") || input.contains("https://");
    let has_domain = input.split_whitespace().any(|w| {
        let w = w.trim_end_matches(|c: char| ".,;:!?)\"'".contains(c));
        w.contains('.') && !w.starts_with('.') && w.len() > 3
            && !w.ends_with(".rs") && !w.ends_with(".md") && !w.ends_with(".toml")
    });

    if has_url || has_domain {
        // Don't launch browser for status/monitoring/info requests about a domain
        let is_info_request = lower.contains("status") || lower.contains("check")
            || lower.contains("monitor") || lower.contains("ping")
            || lower.contains("uptime") || lower.contains("health")
            || lower.contains("tell me") || lower.contains("what is")
            || lower.contains("how is") || lower.contains("is it")
            || lower.contains("services") || lower.contains("running");
        if is_info_request { return AgentIntent::Conversation; }

        // Multi-step browser tasks
        if lower.contains("post") || lower.contains("fill") || lower.contains("submit")
            || lower.contains("login") || lower.contains("sign in")
        {
            return AgentIntent::BrowserAgent;
        }
        // Only browse if intent is clearly to visit/open/navigate
        if lower.contains("open") || lower.contains("go to") || lower.contains("visit")
            || lower.contains("browse") || lower.contains("navigate")
            || lower.starts_with("http") {
            return AgentIntent::BrowserFetch;
        }
        // Default: conversation (let LLM decide if browsing is needed)
        return AgentIntent::Conversation;
    }

    // Shell commands
    if lower.starts_with("run ") || lower.starts_with("execute ") || lower.starts_with("ls ")
        || lower.starts_with("cat ") || lower.starts_with("git ") {
        return AgentIntent::Shell;
    }

    AgentIntent::Conversation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_from_str() {
        assert_eq!(AgentIntent::from_str("browser_agent"), AgentIntent::BrowserAgent);
        assert_eq!(AgentIntent::from_str("desktop"), AgentIntent::Desktop);
        assert_eq!(AgentIntent::from_str("shell"), AgentIntent::Shell);
        assert_eq!(AgentIntent::from_str("action"), AgentIntent::Action);
        assert_eq!(AgentIntent::from_str("task"), AgentIntent::Action);
        assert_eq!(AgentIntent::from_str("conversation"), AgentIntent::Conversation);
        assert_eq!(AgentIntent::from_str("nonsense"), AgentIntent::Conversation);
    }

    #[test]
    fn actionable_intents() {
        assert!(AgentIntent::Action.is_actionable());
        assert!(AgentIntent::Shell.is_actionable());
        assert!(AgentIntent::File.is_actionable());
        assert!(!AgentIntent::Conversation.is_actionable());
        assert!(!AgentIntent::BrowserAgent.is_actionable());
    }

    #[test]
    fn heuristic_url_detection() {
        assert_eq!(classify_heuristic("open https://example.com"), AgentIntent::BrowserFetch);
        assert_eq!(classify_heuristic("post hello on twitter.com"), AgentIntent::BrowserAgent);
        assert_eq!(classify_heuristic("what is rust?"), AgentIntent::Conversation);
    }

    #[test]
    fn inject_enrichments_browser() {
        let mut enrichments = HashMap::new();
        inject_enrichments(&AgentIntent::BrowserAgent, &mut enrichments);
        assert_eq!(enrichments.get("agent_intent").unwrap(), "browser_agent");
        assert_eq!(enrichments.get("browser_relevant").unwrap(), "true");
    }

    #[test]
    fn inject_enrichments_conversation() {
        let mut enrichments = HashMap::new();
        inject_enrichments(&AgentIntent::Conversation, &mut enrichments);
        assert_eq!(enrichments.get("agent_intent").unwrap(), "conversation");
        assert!(enrichments.get("browser_relevant").is_none());
    }
}
