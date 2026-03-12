//! Helper functions for App — extracted from app.rs for file size.

use std::collections::VecDeque;
use super::app::{RecentTask, TaskStatus};

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

pub fn resolve_model_name() -> String {
    let raw = std::env::var("HYDRA_MODEL").unwrap_or_default();
    if raw.is_empty() { return "Sonnet 4.6".into(); }
    let known: &[(&str, &str)] = &[
        ("opus", "Opus 4.6"), ("sonnet", "Sonnet 4.6"), ("haiku", "Haiku 4.5"),
        ("gpt-4o-mini", "GPT-4o Mini"), ("gpt-4o", "GPT-4o"), ("gpt-4.1", "GPT-4.1"),
        ("o3", "o3"), ("o4-mini", "o4-mini"),
        ("gemini-2.5-pro", "Gemini 2.5 Pro"), ("gemini-2.5-flash", "Gemini 2.5 Flash"),
        ("gemini-2.0-flash", "Gemini 2.0 Flash"),
        ("grok-3-mini", "Grok 3 Mini"), ("grok-3", "Grok 3"),
        ("deepseek-r1", "DeepSeek R1"), ("deepseek-v3", "DeepSeek V3"),
        ("mistral-large", "Mistral Large"), ("codestral", "Codestral"),
        ("ollama", "Ollama (local)"),
    ];
    for (pat, name) in known { if raw.contains(pat) { return name.to_string(); } }
    raw
}
