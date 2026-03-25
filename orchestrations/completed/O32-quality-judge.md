# O32: Quality Judge

**Status:** Complete
**Session:** 33
**Built:** 2026-03-25

## What It Does
Evaluates output against goal criteria after task completion. Goes beyond the O5 Quality Critic by checking the RESULT matches the original USER INTENT, not just code quality. If output is incomplete or wrong, triggers remediation steps.

## Crates Used
- hydra-kernel/src/quality_judge.rs (205 lines)

## Dependencies
- Depends on: O5 (Critic — quality dimensions), O27 (Intent Compiler — original intent), O1 (Conductor — remediation steps)
- Required by: AMM agent loop final verification

## Wiring (Law 10)
- Called from: AMM agent loop after task completion, conductor post-execution
- TUI visible: Quality assessment shown in conductor output
- Genome feedback: Quality scores feed back to approach confidence
