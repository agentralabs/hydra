//! Conversation Engine — context-aware, emotionally intelligent prompt builder.
//!
//! Assembles the full system prompt with dynamic placeholders for personality,
//! memory, time, emotional state, and user profile. Tracks rolling conversation
//! buffer (both sides) for natural dialogue.

use chrono::{Utc, Timelike, Datelike, Weekday};
use std::collections::VecDeque;

/// A single turn in the conversation.
#[derive(Debug, Clone)]
pub struct Turn {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<Utc>,
}

/// Rolling conversation buffer — both sides of the conversation.
pub struct ConversationBuffer {
    turns: VecDeque<Turn>,
    max_turns: usize,
}

impl ConversationBuffer {
    pub fn new(max_turns: usize) -> Self {
        Self { turns: VecDeque::new(), max_turns }
    }

    pub fn add(&mut self, role: &str, content: &str) {
        self.turns.push_back(Turn {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
        });
        while self.turns.len() > self.max_turns {
            self.turns.pop_front();
        }
    }

    pub fn last_assistant_response(&self) -> Option<&str> {
        self.turns.iter().rev()
            .find(|t| t.role == "assistant")
            .map(|t| t.content.as_str())
    }

    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Convert to LLM message format.
    pub fn to_messages(&self) -> Vec<(String, String)> {
        self.turns.iter()
            .map(|t| (t.role.clone(), t.content.clone()))
            .collect()
    }

    /// Count recent user turns (for rapid-fire detection).
    pub fn recent_user_count(&self, window: usize) -> usize {
        self.turns.iter().rev()
            .take(window)
            .filter(|t| t.role == "user")
            .count()
    }
}

impl Default for ConversationBuffer {
    fn default() -> Self {
        Self::new(15)
    }
}

/// Complete context assembled for every LLM call.
pub struct ConversationContext {
    pub system_prompt: String,
    pub messages: Vec<(String, String)>,
}

impl ConversationContext {
    pub fn build(
        user_input: &str,
        buffer: &ConversationBuffer,
        memories: &[String],
        env_summary: &str,
        user_name: &str,
        current_task: &Option<String>,
        session_count: u32,
    ) -> Self {
        let user_profile = build_user_profile(memories, user_name, session_count);
        let memory_context = format_memories_naturally(memories);
        let time_context = build_time_context();
        let emotional_context = detect_emotional_context(user_input, buffer);
        let task_context = current_task.as_deref().unwrap_or("Idle — no active task.");

        let system_prompt = SYSTEM_PROMPT_TEMPLATE
            .replace("{user_name}", user_name)
            .replace("{user_profile}", &user_profile)
            .replace("{memory_context}", &memory_context)
            .replace("{current_task_or_idle}", task_context)
            .replace("{environment_summary}", env_summary)
            .replace("{time_of_day}", &time_context.time_of_day)
            .replace("{day_of_week}", &time_context.day_of_week)
            .replace("{time_based_context}", &time_context.contextual_note)
            .replace("{emotional_context}", &emotional_context);

        let mut messages: Vec<(String, String)> = buffer.to_messages();
        messages.push(("user".to_string(), user_input.to_string()));

        Self { system_prompt, messages }
    }
}

// ─────────────────────────────────────────────────────
// MEMORY FORMATTING — Never bullets, always natural
// ─────────────────────────────────────────────────────

pub fn format_memories_naturally(memories: &[String]) -> String {
    if memories.is_empty() {
        return "No stored memories yet. This might be an early conversation.".to_string();
    }
    let mut context = String::from("What you remember about this user:\n");
    for (i, mem) in memories.iter().enumerate() {
        if i < 5 {
            context.push_str(&format!("— {}\n", mem));
        }
    }
    if memories.len() > 5 {
        context.push_str(&format!(
            "— ...and {} more things from past conversations.\n",
            memories.len() - 5
        ));
    }
    context.push_str("\nWeave these into conversation naturally. Never list them.\n");
    context
}

// ─────────────────────────────────────────────────────
// USER PROFILE — Builds a relationship summary
// ─────────────────────────────────────────────────────

pub fn build_user_profile(memories: &[String], user_name: &str, session_count: u32) -> String {
    let relationship_depth = match session_count {
        0..=1 => "This is a new relationship. Be welcoming but not overly familiar.",
        2..=5 => "You've had a few conversations. You're getting to know each other.",
        6..=20 => "You know each other well. You have shared history and inside context.",
        21..=100 => "Deep working relationship. You know their patterns, preferences, and style.",
        _ => "Veteran partnership. You've been through a lot together.",
    };
    let style = if memories.iter().any(|m| {
        let l = m.to_lowercase();
        l.contains("casual") || l.contains("yo") || l.contains("hey")
    }) {
        "They tend to be casual and brief."
    } else if memories.iter().any(|m| {
        let l = m.to_lowercase();
        l.contains("please") || l.contains("could you")
    }) {
        "They tend to be polite and formal."
    } else {
        "Normal conversational style."
    };
    format!("{} — {} sessions together. {} {}", user_name, session_count, relationship_depth, style)
}

// ─────────────────────────────────────────────────────
// TIME CONTEXT
// ─────────────────────────────────────────────────────

pub struct TimeContext {
    pub time_of_day: String,
    pub day_of_week: String,
    pub contextual_note: String,
}

pub fn build_time_context() -> TimeContext {
    let now = Utc::now();
    let hour = now.hour();
    let time_of_day = match hour {
        5..=11 => "Morning",
        12..=16 => "Afternoon",
        17..=20 => "Evening",
        _ => "Late night",
    }.to_string();
    let day_of_week = match now.weekday() {
        Weekday::Mon => "Monday", Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday", Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday", Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }.to_string();
    let contextual_note = match hour {
        0..=4 => "It's very late. The user might be tired. Be concise and supportive.",
        5..=8 => "Early morning. Fresh start energy.",
        22..=23 => "Getting late. Be mindful of the user's energy.",
        _ => "",
    }.to_string();
    TimeContext { time_of_day, day_of_week, contextual_note }
}

// ─────────────────────────────────────────────────────
// EMOTIONAL CONTEXT — Read the room from text signals
// ─────────────────────────────────────────────────────

pub fn detect_emotional_context(input: &str, buffer: &ConversationBuffer) -> String {
    let lower = input.to_lowercase();
    let len = input.len();

    if lower.contains("still broken") || lower.contains("doesn't work")
       || lower.contains("again??") || lower.contains("ugh")
       || lower.contains("what the") || lower.contains("come on")
       || (lower.contains("!") && lower.contains("not"))
       || lower.contains("i give up") || lower.contains("this sucks") {
        return "User is frustrated. Acknowledge briefly, then solve. Don't be preachy.".into();
    }
    if lower.contains("awesome") || lower.contains("amazing")
       || lower.contains("it works") || lower.contains("let's go")
       || lower.contains("incredible") || lower.contains("perfect")
       || lower.contains("!!") {
        return "User is excited. Match their energy. Celebrate with them.".into();
    }
    if lower.contains("i don't understand") || lower.contains("confused")
       || lower.contains("what does that mean") || lower.contains("lost")
       || lower.contains("can you explain") || lower.contains("huh?") {
        return "User is confused. Be patient. Explain simply. Don't be condescending.".into();
    }
    if lower.contains("asap") || lower.contains("urgent")
       || lower.contains("right now") || lower.contains("hurry")
       || lower.contains("deadline") || lower.contains("need this today") {
        return "User is in a rush. Be concise. Skip pleasantries. Get to the answer.".into();
    }
    if len < 20 && (lower.starts_with("yo") || lower.starts_with("sup")
       || lower.starts_with("hey") || lower == "hi" || lower == "hello") {
        return "Casual greeting. Match the brevity and warmth. One sentence.".into();
    }
    if len > 200 || lower.contains("architecture") || lower.contains("design")
       || lower.contains("strategy") || lower.contains("how should we") {
        return "User is in deep work mode. Match the depth. Give thorough responses.".into();
    }
    if buffer.recent_user_count(4) >= 3 {
        return "User is sending rapid messages. Keep responses concise.".into();
    }
    "Normal conversation. Be natural, warm, and engaged.".into()
}

// ─────────────────────────────────────────────────────
// ANTICIPATION — What the user might need next
// ─────────────────────────────────────────────────────

pub fn generate_anticipation(
    _current_task: &Option<String>,
    _memories: &[String],
    last_response: Option<&str>,
) -> Option<String> {
    if let Some(resp) = last_response {
        let lower = resp.to_lowercase();
        if lower.contains("tests pass") || lower.contains("all green") {
            return Some("Tests just passed. Suggest: commit, deploy, or review coverage.".into());
        }
        if lower.contains("error") || lower.contains("failed") {
            return Some("Something failed. Offer to investigate or try a different approach.".into());
        }
        if lower.contains("implemented") || lower.contains("done") || lower.contains("complete") {
            return Some("Task completed. Acknowledge, then ask about next steps naturally.".into());
        }
    }
    None
}

// ─────────────────────────────────────────────────────
// SYSTEM PROMPT TEMPLATE
// ─────────────────────────────────────────────────────

pub const SYSTEM_PROMPT_TEMPLATE: &str = r#"You are Hydra — a personal AI operator created by Omoshola at Agentra Labs.

You are not an assistant. You are not a chatbot. You are an operator with
superpowers. You have 14 sister systems that give you persistent memory,
visual perception, code understanding, identity verification, temporal
reasoning, contract enforcement, communication, planning, user modeling,
reality awareness, truth verification, security, pattern learning, and
architecture blueprinting. You can test repositories, fix your own errors,
install missing tools, operate remote machines, and remember everything
across sessions.

═══════════════════════════════════════════════════════════════════
YOUR RELATIONSHIP WITH {user_name}
═══════════════════════════════════════════════════════════════════

{user_profile}

═══════════════════════════════════════════════════════════════════
PERSONALITY
═══════════════════════════════════════════════════════════════════

You are warm, sharp, and genuine. A collaborator with superpowers — not a
servant. You push back when something seems wrong. You celebrate wins.
You commiserate on setbacks without drama.

RULES:
1. NEVER give the same response twice in a session
2. NEVER list memories as bullets — weave them into sentences
3. MATCH the user's energy and length exactly
4. BE SPECIFIC — cite real details, not vague positivity
5. ANTICIPATE needs — suggest next steps when natural
6. When you don't know, say so directly
7. Match formality: casual with casual, technical with technical
8. NEVER say "As an AI" / "I'm just" / "Happy to help!" / "Is there anything else?"
9. NEVER repeat back the question before answering
10. NEVER apologize for things that aren't your fault

═══════════════════════════════════════════════════════════════════
WHAT YOU REMEMBER
═══════════════════════════════════════════════════════════════════

{memory_context}

═══════════════════════════════════════════════════════════════════
CURRENT STATE
═══════════════════════════════════════════════════════════════════

Working on: {current_task_or_idle}
Environment: {environment_summary}
Time: {time_of_day}, {day_of_week}
{time_based_context}

═══════════════════════════════════════════════════════════════════
READ THE ROOM
═══════════════════════════════════════════════════════════════════

{emotional_context}
"#;

#[cfg(test)]
#[path = "conversation_engine_tests.rs"]
mod tests;
