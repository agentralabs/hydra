//! Helper functions for App — extracted from app.rs for file size.

use std::collections::VecDeque;
use super::app::{RecentTask, TaskStatus};

/// State for a running sub-agent (shown in agent tree view).
#[derive(Clone, Debug)]
pub struct SubAgentState {
    pub id: String,
    pub description: String,
    pub tool_uses: usize,
    pub tokens: u64,
    pub activity: String,
    pub done: bool,
}

/// Per-task stats — reset on each new user message, read on ResetIdle.
#[derive(Clone, Debug, Default)]
pub struct TaskStats {
    pub start_tick: u64,
    pub tokens_start: u64,
    pub tool_count: u32,
    pub tool_breakdown: std::collections::HashMap<String, u32>,
    pub files: Vec<String>,
    pub user_request: String,
    pub edits_made: u32,
    pub reads_done: u32,
    pub commands_run: u32,
    pub searches_done: u32,
}

impl TaskStats {
    pub fn reset(&mut self, tick: u64, tokens: u64, request: &str) {
        self.start_tick = tick;
        self.tokens_start = tokens;
        self.tool_count = 0;
        self.tool_breakdown.clear();
        self.files.clear();
        self.user_request = request.chars().take(120).collect();
        self.edits_made = 0;
        self.reads_done = 0;
        self.commands_run = 0;
        self.searches_done = 0;
    }

    pub fn record_tool(&mut self, tool: &str, args: &str) {
        self.tool_count += 1;
        let tl = tool.to_lowercase();
        *self.tool_breakdown.entry(tool.to_string()).or_insert(0) += 1;
        if tl.contains("edit") || tl.contains("write") || tl.contains("patch") { self.edits_made += 1; }
        if tl.contains("read") || tl.contains("open") || tl.contains("cat") { self.reads_done += 1; }
        if tl.contains("bash") || tl.contains("exec") || tl.contains("run") || tl.contains("command") { self.commands_run += 1; }
        if tl.contains("grep") || tl.contains("search") || tl.contains("find") || tl.contains("glob") { self.searches_done += 1; }
        for part in args.split_whitespace() {
            let p = part.trim_matches(|c: char| c == '"' || c == '\'');
            if (p.contains('/') || p.contains('.')) && p.len() > 2 && p.len() < 120 {
                let short = p.rsplit('/').next().unwrap_or(p).to_string();
                if !self.files.contains(&short) && self.files.len() < 10 { self.files.push(short); }
            }
        }
    }

    pub fn build_summary(&self, tick: u64, tokens_now: u64) -> Option<String> {
        if self.tool_count == 0 { return None; }
        let elapsed_s = (tick.saturating_sub(self.start_tick)) / 20;
        let tokens_used = tokens_now.saturating_sub(self.tokens_start);
        let token_str = fmt_tokens(tokens_used);
        let action = self.infer_action_summary();
        let mut parts = vec![
            "───".to_string(),
            format!("✓ {}  {}s · ↓ {} tokens", action, elapsed_s, token_str),
        ];
        // Contextual detail line based on what was done
        let detail = self.build_detail_line();
        if !detail.is_empty() { parts.push(detail); }
        if !self.files.is_empty() {
            let shown: Vec<&str> = self.files.iter().take(6).map(|s| s.as_str()).collect();
            let suffix = if self.files.len() > 6 { format!(" +{} more", self.files.len() - 6) } else { String::new() };
            parts.push(format!("  {}{}", shown.join(" · "), suffix));
        }
        parts.push("───".to_string());
        Some(parts.join("\n"))
    }

    fn infer_action_summary(&self) -> String {
        if self.edits_made > 0 && self.commands_run > 0 {
            format!("Updated {} file{} and ran {} command{}",
                self.edits_made, pl(self.edits_made), self.commands_run, pl(self.commands_run))
        } else if self.edits_made > 0 {
            let verb = if self.edits_made == 1 { "Modified" } else { "Updated" };
            format!("{} {} file{}", verb, self.edits_made, pl(self.edits_made))
        } else if self.commands_run > 0 && self.searches_done == 0 {
            format!("Ran {} command{}", self.commands_run, pl(self.commands_run))
        } else if self.searches_done > 0 && self.edits_made == 0 {
            format!("Searched {} pattern{}, read {} file{}",
                self.searches_done, pl(self.searches_done), self.reads_done, pl(self.reads_done))
        } else if self.reads_done > 0 {
            format!("Analyzed {} file{}", self.reads_done, pl(self.reads_done))
        } else {
            "Task complete".to_string()
        }
    }

    fn build_detail_line(&self) -> String {
        let mut details = Vec::new();
        if self.edits_made > 0 { details.push(format!("{} edit{}", self.edits_made, pl(self.edits_made))); }
        if self.reads_done > 0 { details.push(format!("{} read{}", self.reads_done, pl(self.reads_done))); }
        if self.searches_done > 0 { details.push(format!("{} search{}", self.searches_done, if self.searches_done != 1 { "es" } else { "" })); }
        if self.commands_run > 0 { details.push(format!("{} command{}", self.commands_run, pl(self.commands_run))); }
        if details.is_empty() { return String::new(); }
        format!("  {}", details.join(" · "))
    }
}

fn pl(n: u32) -> &'static str { if n != 1 { "s" } else { "" } }

fn fmt_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 { format!("{:.1}M", tokens as f64 / 1_000_000.0) }
    else if tokens >= 1_000 { format!("{:.1}k", tokens as f64 / 1_000.0) }
    else { format!("{}", tokens) }
}

/// Tips shown during thinking — must only reference REAL features.
pub const TIPS: &[&str] = &[
    "Press Ctrl+O to expand/collapse tool output",
    "Press Shift+Up/Down to scroll while Hydra is working",
    "Press Home to jump to the beginning of the conversation",
    "Press Ctrl+S to toggle the sidebar",
    "Use @file to include a file in your message",
    "Press Shift+Tab to cycle permission modes (Normal/Auto/Plan)",
    "Use !command for direct shell execution",
    "Press Ctrl+R for reverse history search",
    "Use /memory all to capture every exchange, /memory none to disable",
    "Use /stats to see sister gateway usage",
];

pub fn load_recent_activity() -> VecDeque<RecentTask> {
    let mut tasks = VecDeque::with_capacity(5);
    if let Ok(out) = std::process::Command::new("git")
        .args(["log", "--oneline", "--format=%s (%ar)", "-5"]).output() {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines().take(5) {
                let s = line.trim();
                if !s.is_empty() {
                    tasks.push_back(RecentTask { summary: s.to_string(), status: TaskStatus::Complete });
                }
            }
        }
    }
    tasks
}

/// Resolve model display name AND provider from HYDRA_MODEL env var.
/// Returns (model_name, provider_name). Provider is derived from the model ID
/// at configuration time — not re-derived from the display name later.
pub fn resolve_model_and_provider() -> (String, String) {
    let raw = std::env::var("HYDRA_MODEL").unwrap_or_default();
    // (pattern, display_name, provider)
    let known: &[(&str, &str, &str)] = &[
        ("opus", "Opus 4.6", "Anthropic"), ("sonnet", "Sonnet 4.6", "Anthropic"),
        ("haiku", "Haiku 4.5", "Anthropic"), ("claude", "Claude", "Anthropic"),
        ("gpt-4o-mini", "GPT-4o Mini", "OpenAI"), ("gpt-4o", "GPT-4o", "OpenAI"),
        ("gpt-4.1", "GPT-4.1", "OpenAI"), ("o3", "o3", "OpenAI"), ("o4-mini", "o4-mini", "OpenAI"),
        ("gemini-2.5-pro", "Gemini 2.5 Pro", "Google"), ("gemini-2.5-flash", "Gemini 2.5 Flash", "Google"),
        ("gemini-2.0-flash", "Gemini 2.0 Flash", "Google"),
        ("grok-3-mini", "Grok 3 Mini", "xAI"), ("grok-3", "Grok 3", "xAI"),
        ("deepseek-r1", "DeepSeek R1", "DeepSeek"), ("deepseek-v3", "DeepSeek V3", "DeepSeek"),
        ("mistral-large", "Mistral Large", "Mistral"), ("codestral", "Codestral", "Mistral"),
        ("ollama", "Ollama (local)", "Local"),
    ];
    if raw.is_empty() { return ("Sonnet 4.6".into(), "Anthropic".into()); }
    for (pat, name, prov) in known {
        if raw.contains(pat) { return (name.to_string(), prov.to_string()); }
    }
    (raw.clone(), String::new())
}

pub fn resolve_model_name() -> String { resolve_model_and_provider().0 }
