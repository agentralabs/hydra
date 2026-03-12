# SPEC: Error Recovery Patterns

## Problem
When Hydra encounters errors (HTTP failures, LLM timeouts, parse errors), it just reports the error and stops. It doesn't try alternative approaches, retry with backoff, or degrade gracefully.

## Requirement
Create an error recovery utility that:
- Classifies errors by type (network, parse, timeout, auth, rate_limit)
- Suggests recovery action per error type
- Implements simple retry with exponential backoff (configurable max retries)
- Tracks error frequency to detect recurring issues

## Acceptance Criteria
1. `pub enum ErrorKind` — Network, Parse, Timeout, Auth, RateLimit, Unknown
2. `pub fn classify_error(error_msg: &str) -> ErrorKind` — pattern match on error text
3. `pub fn recovery_action(kind: ErrorKind) -> &'static str` — what to do
4. `pub fn backoff_delay(attempt: u32, base_ms: u64) -> u64` — exponential backoff with jitter
5. `pub struct ErrorTracker` — counts errors by kind
6. `pub fn track(&mut self, kind: ErrorKind)` and `pub fn most_common(&self) -> Option<(ErrorKind, usize)>`
7. Unit tests for classify, backoff calculation, and tracker

## Implementation Location
- New file: `crates/hydra-kernel/src/error_recovery.rs`
