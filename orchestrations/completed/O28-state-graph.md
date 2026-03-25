# O28: State Graph

**Status:** Complete
**Session:** 33
**Built:** 2026-03-25

## What It Does
Learns the state machine of each application Hydra interacts with. Predicts consequences of actions before execution ("if I click Delete, the file is gone"). Builds a graph of app states and transitions from experience.

## Crates Used
- hydra-desktop/src/state_graph.rs (173 lines)

## Dependencies
- Depends on: O26 (AMM — app model for state observation)
- Required by: O29 (Autonomy — uses state predictions for risk assessment)

## Wiring (Law 10)
- Called from: AMM verification layer queries state predictions
- TUI visible: State predictions logged in agent task output
- Genome feedback: Correct predictions increase state transition confidence
