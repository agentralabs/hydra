# SPEC: Self-Improvement Decision Engine

## Problem
Hydra can only self-modify when a user explicitly provides a spec file. It cannot decide on its own what to improve. After a conversation, Hydra should be able to reflect on what went wrong and generate its own improvement specs.

## Requirement
Create a self-improvement engine that:
- Takes a conversation history (user messages + hydra responses + errors)
- Uses LLM to analyze: what went wrong? what could be better?
- Generates a structured improvement suggestion (not a full spec, but a proposal)
- Ranks suggestions by impact (high/medium/low)
- Stores suggestions for later review

## Acceptance Criteria
1. `pub struct ImprovementSuggestion` with fields: description, category (bug_fix, feature, performance, ux), impact (high/medium/low), target_area (conversation, pipeline, routing, sisters)
2. `pub fn analyze_conversation(history: &[(String, String)]) -> Vec<ImprovementSuggestion>` — offline analysis using pattern matching (no LLM)
3. `pub fn detect_errors(history: &[(String, String)]) -> Vec<String>` — extract error messages from history
4. `pub fn rank_suggestions(suggestions: &mut Vec<ImprovementSuggestion>)` — sort by impact
5. `pub fn format_as_spec(suggestion: &ImprovementSuggestion) -> String` — convert suggestion to markdown spec format
6. Unit tests

## Implementation Location
- New file: `crates/hydra-kernel/src/self_improve.rs`
