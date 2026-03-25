# O31: Proactive Agent

**Status:** Complete
**Session:** 33
**Built:** 2026-03-25

## What It Does
Watches for triggers and initiates work WITHOUT a human prompt. Detects patterns like "file changed → run tests", "deadline approaching → remind user", "error rate rising → investigate". Turns Hydra from reactive to anticipatory.

## Crates Used
- hydra-kernel/src/proactive.rs (161 lines)

## Dependencies
- Depends on: O16 (Monitor — trigger sources), O15 (Collaboration — file watching), O21 (User Model — user patterns)
- Required by: Ambient loop proactive checks

## Wiring (Law 10)
- Called from: Ambient loop periodic checks
- TUI visible: Proactive suggestions appear as system notifications
- Genome feedback: Accepted suggestions strengthen trigger patterns
