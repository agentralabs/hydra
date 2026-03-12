//! Multi-turn LLM orchestration — agentic loop inside the LLM.
//!
//! Phase 4, Part A: Instead of single prompt → single response, this runs
//! a conversation with the LLM where it reasons, calls tools, sees results,
//! and continues until the task is done (or limits hit).

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Role for a session turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnRole {
    User,
    Assistant,
    ToolResult,
    System,
}

/// A tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub call_id: String,
}

/// A tool available to the LLM session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// A single turn in the multi-turn session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    pub turn: usize,
    pub role: TurnRole,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub tokens_used: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Result of a completed session.
#[derive(Debug, Clone)]
pub enum SessionResult {
    /// Task completed successfully.
    Complete {
        answer: String,
        turns: usize,
        tokens: u32,
    },
    /// Token budget exhausted before completion.
    BudgetExhausted {
        turns: usize,
        partial_answer: String,
    },
    /// Maximum turns reached without completion.
    MaxTurnsReached {
        turns: usize,
        partial_answer: String,
    },
    /// Session failed with an error.
    Error {
        message: String,
        turns: usize,
    },
}

impl SessionResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Complete { .. })
    }

    pub fn answer(&self) -> &str {
        match self {
            Self::Complete { answer, .. } => answer,
            Self::BudgetExhausted { partial_answer, .. } => partial_answer,
            Self::MaxTurnsReached { partial_answer, .. } => partial_answer,
            Self::Error { message, .. } => message,
        }
    }

    pub fn turns_used(&self) -> usize {
        match self {
            Self::Complete { turns, .. }
            | Self::BudgetExhausted { turns, .. }
            | Self::MaxTurnsReached { turns, .. }
            | Self::Error { turns, .. } => *turns,
        }
    }
}

/// Configuration for an agentic session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub max_turns: usize,
    pub turn_timeout_secs: u64,
    pub total_budget_tokens: u32,
    pub temperature: f64,
    pub system_prompt: Option<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_turns: 20,
            turn_timeout_secs: 30,
            total_budget_tokens: 40_000,
            temperature: 0.3,
            system_prompt: None,
        }
    }
}

/// Multi-turn LLM session — runs an agentic loop inside the LLM.
/// Not one prompt. A conversation where the LLM reasons, uses tools,
/// sees results, and continues until the task is done.
pub struct AgenticSession {
    pub session_id: String,
    pub config: SessionConfig,
    pub history: Vec<SessionTurn>,
    tokens_used: u32,
    started_at: Option<Instant>,
}

impl AgenticSession {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            config,
            history: Vec::new(),
            tokens_used: 0,
            started_at: None,
        }
    }

    /// Add a turn to the session history.
    pub fn add_turn(&mut self, role: TurnRole, content: String, tokens: u32) {
        let turn = SessionTurn {
            turn: self.history.len(),
            role,
            content,
            tool_calls: Vec::new(),
            tokens_used: tokens,
            timestamp: chrono::Utc::now(),
        };
        self.tokens_used += tokens;
        self.history.push(turn);
    }

    /// Add a turn with tool calls.
    pub fn add_turn_with_tools(
        &mut self,
        role: TurnRole,
        content: String,
        tool_calls: Vec<ToolCall>,
        tokens: u32,
    ) {
        let turn = SessionTurn {
            turn: self.history.len(),
            role,
            content,
            tool_calls,
            tokens_used: tokens,
            timestamp: chrono::Utc::now(),
        };
        self.tokens_used += tokens;
        self.history.push(turn);
    }

    /// Total tokens used so far.
    pub fn tokens_used(&self) -> u32 {
        self.tokens_used
    }

    /// Number of turns completed.
    pub fn turns_completed(&self) -> usize {
        self.history.len()
    }

    /// Whether the budget has been exhausted.
    pub fn budget_exhausted(&self) -> bool {
        self.tokens_used >= self.config.total_budget_tokens
    }

    /// Whether the max turns have been reached.
    pub fn max_turns_reached(&self) -> bool {
        self.history.len() >= self.config.max_turns
    }

    /// Build the message history for the LLM call.
    /// Converts session turns into the provider's message format.
    pub fn build_messages(&self) -> Vec<(String, String)> {
        self.history
            .iter()
            .map(|turn| {
                let role = match turn.role {
                    TurnRole::User => "user",
                    TurnRole::Assistant => "assistant",
                    TurnRole::ToolResult => "user", // tool results sent as user messages
                    TurnRole::System => "system",
                };
                (role.to_string(), turn.content.clone())
            })
            .collect()
    }

    /// Get the last assistant response (the most recent answer).
    pub fn last_answer(&self) -> Option<&str> {
        self.history
            .iter()
            .rev()
            .find(|t| t.role == TurnRole::Assistant)
            .map(|t| t.content.as_str())
    }

    /// Check if the LLM's response indicates task completion.
    /// The LLM signals completion by NOT requesting any tool calls
    /// and providing a final answer.
    pub fn is_final_response(response_content: &str, tool_calls: &[ToolCall]) -> bool {
        // If there are tool calls, it's not done yet
        if !tool_calls.is_empty() {
            return false;
        }
        // If the response is empty or very short, probably not a final answer
        if response_content.len() < 5 {
            return false;
        }
        true
    }

    /// Mark the session as started.
    pub fn start(&mut self) {
        self.started_at = Some(Instant::now());
    }

    /// Elapsed time since session started.
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Determine whether a task should use multi-turn agentic session
/// vs single-prompt path.
///
/// Routing rule from the spec:
/// - Task contains multiple steps → agentic session
/// - Task requires tool use across multiple results → agentic session
/// - Simple greeting/question → single prompt
pub fn should_use_agentic_session(text: &str) -> bool {
    let lower = text.to_lowercase();

    // Multi-step indicators
    let multi_step = lower.contains(" then ")
        || lower.contains(" and then ")
        || lower.contains(" after that ")
        || lower.contains(", then ")
        || lower.contains("step 1")
        || lower.contains("first ")  && (lower.contains(" then ") || lower.contains(" next "))
        || lower.contains("implement this spec")
        || lower.contains("build this yourself")
        || lower.contains("upgrade yourself");

    // Tool-use-across-results indicators
    let needs_multi_tool = lower.contains("search for") && lower.contains("then")
        || lower.contains("find") && lower.contains("understand") && lower.contains("write")
        || lower.contains("analyze") && lower.contains("generate")
        || lower.contains("read") && lower.contains("modify");

    // Explicit complexity
    let explicit_complex = lower.contains("build a")
        && (lower.contains("with") || lower.contains("and") || lower.contains("including"));

    multi_step || needs_multi_tool || explicit_complex
}

#[cfg(test)]
#[path = "orchestration_tests.rs"]
mod tests;
