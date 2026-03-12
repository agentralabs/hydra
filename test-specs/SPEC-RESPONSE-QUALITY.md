# SPEC: Response Quality Scorer

## Problem
Hydra has no way to evaluate whether its own responses are good. It can't tell if a response was too generic, too long, didn't answer the question, or had errors. This makes it impossible to learn from mistakes.

## Requirement
Create a response quality scorer that evaluates Hydra's responses using heuristics:
- Length appropriateness (not too short for complex questions, not too long for simple ones)
- Error detection (response contains "error", "failed", "cannot")
- Relevance check (response keywords overlap with question keywords)
- Specificity score (contains code, file paths, or concrete suggestions vs vague advice)
- Personality score (friendly, uses user's name, not robotic)

## Acceptance Criteria
1. `pub struct QualityScore` with fields: length_score (0.0-1.0), error_free (bool), relevance (0.0-1.0), specificity (0.0-1.0), personality (0.0-1.0), overall (0.0-1.0)
2. `pub fn score_response(question: &str, response: &str, user_name: &str) -> QualityScore`
3. `pub fn is_too_generic(response: &str) -> bool` — detects boilerplate responses
4. `pub fn suggest_improvement(score: &QualityScore) -> Option<String>` — what to do better
5. Unit tests with example good and bad responses

## Implementation Location
- New file: `crates/hydra-kernel/src/response_quality.rs`
