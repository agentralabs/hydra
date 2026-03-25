# O27: Intent Compiler

**Status:** Complete
**Session:** 33
**Built:** 2026-03-25

## What It Does
Compiles natural language user intent ("draw a rectangle in AutoCAD") into typed, optimized UI action plans. Parse → Resolve → Optimize → Emit pipeline that bridges human language to precise application commands.

## Crates Used
- hydra-kernel/src/intent_compiler.rs (212 lines)

## Dependencies
- Depends on: O26 (AMM — app model for resolution), O1 (Conductor — step decomposition)
- Required by: O30 (Recovery — recompiles plans after failure)

## Wiring (Law 10)
- Called from: AMM agent loop uses compiled intents for step planning
- TUI visible: Compiled steps shown in conductor output
- Genome feedback: Successful compilations strengthen approach confidence
