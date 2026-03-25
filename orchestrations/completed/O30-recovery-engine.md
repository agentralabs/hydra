# O30: Recovery Engine

**Status:** Complete
**Session:** 33
**Built:** 2026-03-25

## What It Does
When an action fails, classifies the failure type, looks up known recovery patterns from the genome, and recompiles the remaining plan. Turns failures into learning opportunities instead of dead ends.

## Crates Used
- hydra-kernel/src/recovery.rs (214 lines)

## Dependencies
- Depends on: O27 (Intent Compiler — recompiles plans), O3 (Feedback — records recovery outcomes)
- Required by: O26 (AMM — recovery after failed UI actions)

## Wiring (Law 10)
- Called from: AMM agent loop on step failure
- TUI visible: Recovery attempts shown in conductor step output
- Genome feedback: Successful recoveries create new genome entries for that failure pattern
