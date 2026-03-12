# SPEC: Uptime Tracker Utility

## Requirement
Create a new standalone utility module that tracks how long Hydra has been running.
- Record the startup timestamp
- Provide a function to get human-readable uptime (e.g., "2h 15m 30s")
- Provide a function to get uptime in seconds

## Acceptance Criteria
1. New file created: `crates/hydra-kernel/src/uptime.rs`
2. `pub fn start() -> Instant` — records startup time
3. `pub fn format_uptime(start: Instant) -> String` — returns "Xh Ym Zs"
4. `pub fn uptime_secs(start: Instant) -> u64` — returns seconds
5. Unit tests included in the same file

## Implementation Location
- New file: `crates/hydra-kernel/src/uptime.rs`
- Register in: `crates/hydra-kernel/src/lib.rs` (add `pub mod uptime;`)
