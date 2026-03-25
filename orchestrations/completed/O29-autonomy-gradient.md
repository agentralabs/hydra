# O29: Autonomy Gradient

**Status:** Complete
**Session:** 33
**Built:** 2026-03-25

## What It Does
Replaces the binary act/ask/refuse judgment gate with a continuous 0-1 autonomy score. Computed from: confidence × reversibility × blast radius × history. Maps to 4 decisions: ActSilently (>0.8), ActAndNotify (0.5-0.8), AskFirst (0.2-0.5), Refuse (<0.2).

## Crates Used
- hydra-wisdom/src/autonomy.rs (173 lines)

## Dependencies
- Depends on: O3 (Feedback — genome confidence), O28 (State Graph — reversibility prediction)
- Required by: O6 (Worker — replaces hardcoded trust_score)

## Wiring (Law 10)
- Called from: worker.rs:autonomy_check() — wired into every conductor step execution
- TUI visible: Autonomy score logged per step (eprintln "hydra-autonomy: X → 0.85")
- Genome feedback: Successful autonomous actions increase genome confidence for that pattern
