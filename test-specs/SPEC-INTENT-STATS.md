# SPEC: Intent Classification Statistics

## Requirement
Add a simple intent statistics counter that tracks how many times each intent category has been classified. This helps debug intent routing issues.

- Count classifications per category (Greeting, CodeBuild, SelfImplement, etc.)
- Provide a function to get the top-N most common intents
- Thread-safe (uses AtomicU64 or Mutex)

## Acceptance Criteria
1. New file created: `crates/hydra-kernel/src/intent_stats.rs`
2. `pub struct IntentStats` with a `HashMap<String, AtomicU64>`
3. `pub fn record(&self, category: &str)` — increment count
4. `pub fn top_n(&self, n: usize) -> Vec<(String, u64)>` — sorted descending
5. `pub fn total(&self) -> u64` — total classifications
6. Unit tests for record, top_n, and total

## Implementation Location
- New file: `crates/hydra-kernel/src/intent_stats.rs`
