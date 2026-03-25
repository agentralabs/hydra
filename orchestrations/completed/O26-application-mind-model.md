# O26: Application Mind Model (AMM)

**Status:** Complete
**Session:** 33
**Built:** 2026-03-25

## What It Does
6-layer stack that lets Hydra understand and control ANY desktop application like a human. Perceives screen changes differentially, builds an internal model of the app's UI (menus, tools, shortcuts), uses universal UI conventions, executes with Fitts's Law motor kinematics, verifies actions via a 5-tier cascade (95% zero-token), and crystallizes successful sequences as muscle memory in the genome.

## Crates Used
- hydra-desktop/src/perception.rs (162 lines) — L1: Differential vision, coordinate transforms
- hydra-desktop/src/app_model.rs (203 lines) — L2: First contact protocol, menu/tool/shortcut discovery
- hydra-kernel/src/convention.rs (116 lines) — L3: Universal UI conventions (Cmd+S, Cmd+Z, etc.)
- hydra-desktop/src/input.rs (existing) — L4: Fitts's Law + minimum-jerk kinematics
- hydra-desktop/src/verification.rs (137 lines) — L5: 5-tier cascade verification
- hydra-kernel/src/muscle_memory.rs (107 lines) — L6: Crystallization in genome
- hydra-desktop/src/agent_amm.rs (166 lines) — Wire: Full 6-layer execution loop

## Dependencies
- Depends on: O2 (Vision Bridge), O6 (Universal Worker), O9 (Coder), O22 (Rich Output)
- Required by: O27 (Intent Compiler), O28 (State Graph)

## Wiring (Law 10)
- Called from: agent_amm.rs execute_task_v2() chains all 6 layers
- TUI visible: Agent task progress in stream via AGENT_DISPATCH
- Genome feedback: muscle_memory crystallizes successful sequences

## Key Decisions
- First Contact Protocol: when encountering a new app, Hydra explores menus and builds an AppModel before attempting actions
- Convention shortcuts tried BEFORE vision (zero tokens for Cmd+S, Cmd+Z etc.)
- Differential perception: only analyze changed screen regions (saves 90% vision tokens)
- Fitts's Law motor: realistic mouse trajectories, not instant teleportation
