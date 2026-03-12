# SPEC: Conversation Context Memory

## Problem
Hydra has no memory between messages in a conversation. Each message is processed independently. When the user says "can you implement that on yourself?" after discussing improvements, Hydra doesn't know what "that" refers to.

## Requirement
Create a lightweight conversation context tracker that:
- Maintains a sliding window of the last N messages (configurable, default 10)
- Extracts key topics from each message (nouns, verbs, entities)
- Tracks the current conversation "thread" (what topic is being discussed)
- Provides a summary string that can be injected into LLM prompts for context

## Acceptance Criteria
1. `pub struct ConversationContext` with message history and extracted topics
2. `pub fn new(window_size: usize) -> Self`
3. `pub fn add_message(&mut self, role: &str, content: &str)` — add and extract topics
4. `pub fn current_topic(&self) -> Option<String>` — most recent/frequent topic
5. `pub fn context_summary(&self) -> String` — 2-3 sentence summary for LLM injection
6. `pub fn references_previous(&self, text: &str) -> bool` — detects "that", "it", "this", "those" referring to prior messages
7. Unit tests

## Implementation Location
- New file: `crates/hydra-kernel/src/conversation_context.rs`
