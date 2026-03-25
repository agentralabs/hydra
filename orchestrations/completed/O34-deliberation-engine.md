# O34: Deliberation Engine — Adaptive Thinking Before Acting

## Summary
Every AI agent thinks the same amount about every task. The Deliberation Engine
adapts thinking depth to task difficulty using the depth function:

  depth = complexity × (1 - confidence) × novelty

Simple tasks skip thinking. Complex novel tasks get deep research + multiple
plan revision cycles. No system has ever adapted thinking depth to task difficulty.

## Key Files
- `crates/hydra-kernel/src/deliberation.rs` — 5 cognitive modes + state machine + depth function
- `crates/hydra-kernel/src/engine.rs` — Wired before LLM call on every cycle

## The 5 Cognitive Modes

```
ASSESS → "Do I know enough?"
  ├→ confident → PLAN
  └→ gaps → RESEARCH

RESEARCH → web search + genome query
  └→ findings → PLAN

PLAN → intent compiler + research context
  └→ steps → CRITIQUE

CRITIQUE → self-review
  ├→ sound → EXECUTE
  ├→ gaps → RESEARCH
  └→ flawed → PLAN (revise)

EXECUTE → AMM 6-layer + O33 atoms + O30 recovery + O32 quality
```

## Depth Function Behavior
| Task | Complexity | Confidence | Novelty | Depth | Behavior |
|---|---|---|---|---|---|
| "save file" | 0.2 | 0.9 | 0.2 | 0.004 | Skip thinking, just Cmd+S |
| "write a test" | 0.5 | 0.8 | 0.5 | 0.05 | Quick plan then execute |
| "design database" | 0.8 | 0.3 | 0.6 | 0.34 | Assess + Plan + Critique |
| "build trading algo" | 0.8 | 0.1 | 1.0 | 0.72 | Deep Research + multi-revision |

## TUI Visibility
```
hydra thinking...
├ [ASSESS] Creative domain: 40% confidence — need research
├ [RESEARCH] Searching: "floor plan design principles"
│   Found: min room sizes, building codes, circulation
├ [PLAN] 8 steps: walls → rooms → kitchen → bath → doors → windows
├ [CRITIQUE] Missing bathroom ventilation. Revising.
├ [PLAN] 9 steps (added ventilation check)
├ [CRITIQUE] Plan sound. Proceeding.
└ [EXECUTE] Starting...
```

## Integration
- Runs BEFORE every LLM call in engine.rs cycle()
- Research findings injected as `deliberation.research` enrichment
- Plan injected as `deliberation.plan` enrichment
- Thinking log emitted to stderr for TUI consumption
- Max iterations prevent infinite thinking loops (2-6 depending on depth)

## Session: 33 (2026-03-25)
