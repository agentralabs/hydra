# SPEC: Health Summary Generator

## Requirement
Create a health summary function that aggregates system state into a single struct. Used by `/health` and the status sidebar.

- Collect: sister count (connected/total), memory usage estimate, uptime, last error
- Return a serializable struct
- No async, no sister calls — just aggregates data passed to it

## Acceptance Criteria
1. New file created: `crates/hydra-kernel/src/health_summary.rs`
2. `pub struct HealthSummary` with fields: connected_sisters, total_sisters, uptime_secs, last_error, memory_mb
3. `pub fn generate(connected: usize, total: usize, uptime: u64, last_err: Option<String>) -> HealthSummary`
4. `impl Display for HealthSummary` — human-readable multi-line output
5. Unit tests for generate and Display

## Implementation Location
- New file: `crates/hydra-kernel/src/health_summary.rs`
