# 09 — How Hydra Survives

## Hydra Cannot Die

If the system crashes, Hydra comes back warm. If agents are destroyed, new ones spawn. If the substrate changes, the identity transfers. This is not aspiration — it is architecture.

## Resurrection (hydra-resurrection)

Delta-state checkpointing ensures Hydra never starts cold:

```
Every 100 ambient ticks:
  1. Capture current state as KernelStateSnapshot
  2. Compute delta from last checkpoint
  3. SHA256 hash the payload
  4. Store: Full checkpoint (first) or Delta (subsequent)

On crash recovery:
  1. Find the last Full checkpoint
  2. Apply all subsequent Deltas in order
  3. Skip any corrupted Deltas (detected by hash mismatch)
  4. Resume from reconstructed state

Warm restart target: < 2 seconds
Corrupted checkpoints: skipped silently, never crash
```

The checkpoint writer decides Full vs Delta automatically. After a configurable number of Deltas, a new Full is forced.

## Metabolism (hydra-metabolism)

The Lyapunov stability monitor — Hydra's long-term health guardian:

```
Stability classes:
  Optimal:   V(Ψ) ≥ 0.3   — healthy, all systems nominal
  Stable:    V(Ψ) ≥ 0.0   — acceptable, minor concerns
  Alert:     V(Ψ) ≥ -0.5  — intervention level 1
  Critical:  V(Ψ) ≥ -1.0  — intervention level 2
  Emergency: V(Ψ) < -1.0  — intervention level 3

Intervention levels:
  Level 1 (Alert):     log warning, increase checkpoint frequency
  Level 2 (Critical):  reduce fleet size, pause non-essential tasks
  Level 3 (Emergency): full state dump, request human intervention
```

The metabolism monitor also tracks the growth invariant: `Γ̂ ≥ 0`. If growth rate goes negative (capabilities decreasing), the monitor raises an error. Hydra is not allowed to lose capabilities.

Trend detection uses linear regression over the last 100 Lyapunov values. Positive slope = improving. Negative slope = degrading. The trend fires before the value reaches a threshold — Hydra knows it is getting worse before it is actually bad.

## Continuity (hydra-continuity)

The morphic signature proves that the entity running today is the same entity that started on day one:

```
Day 1:   genesis hash → H₀
Day 2:   H₀ + event₁ → H₁
Day 30:  H₂₉ + event₃₀ → H₃₀
Year 10: H₃₆₅₀ + event₃₆₅₁ → H₃₆₅₁

Every hash builds on the previous one.
The chain is unforgeable.
Break any link → the chain does not verify.
```

Yearly checkpoints prove lineage. Succession verification (new hardware, new substrate) proves the transferred entity is the same entity.

## Succession (hydra-succession)

When Hydra moves to new hardware or a new version:

```
EXPORT:
  1. Soul orientation (what the work is for)
  2. Genome (proven approaches with confidence scores)
  3. Calibration profiles (where judgment goes wrong)
  4. Morphic signature (unforgeable identity chain)

VERIFY (3 gates):
  1. Integrity — SHA256 of the entire package
  2. Identity — morphic signature matches lineage
  3. Constitution — 7 laws still intact

IMPORT:
  One-time per instance. Immutable once imported.
  The entity survives the substrate change.
```

## Legacy (hydra-legacy)

What Hydra learned escapes to the world. Three kinds of permanent artifact:

- **Knowledge records** — soul orientation distilled into shareable form
- **Operational records** — proven approaches documented
- **Wisdom records** — cross-domain calibration insights

Every artifact is SHA256 verified and immutable.

## Influence (hydra-influence)

After 20 years of operation, Hydra's proven patterns become the starting point for others:

```
Publish a proven pattern:
  - Source lineage (which Hydra proved this)
  - Category (engineering, security, deployment, etc.)
  - Evidence count (how many times it was observed)
  - Confidence (Bayesian posterior after N observations)

Other Hydras discover and adopt it:
  - Adoption record tracks success/failure
  - Outcome feeds back to the publishing Hydra
  - Successful patterns spread. Failed ones are flagged.
```

This is evolution. Patterns that work survive. Patterns that don't are marked.

## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-resurrection` | 856 | Delta checkpointing, warm restart, corruption recovery |
| `hydra-metabolism` | 712 | Lyapunov stability monitoring, intervention levels |
| `hydra-continuity` | 875 | Morphic signature persistence, lineage proofs |
| `hydra-succession` | 1,311 | Entity transfer across hardware/versions |
| `hydra-legacy` | 632 | Permanent knowledge export |
| `hydra-influence` | 1,020 | Pattern publication and adoption across instances |

## In Plain Terms

Imagine a person who:
- Takes a snapshot of their mental state every few seconds (resurrection)
- Monitors their own health continuously and intervenes before problems become crises (metabolism)
- Has an unbreakable identity chain proving they are the same person they were 20 years ago (continuity)
- Can move to a new body and bring all their knowledge, skills, and identity with them (succession)
- Writes books that other people can learn from, and tracks whether the advice worked (legacy + influence)

That is how Hydra survives. Not just crashes. Not just hardware changes. Time itself. Hydra is designed to be a 20-year entity.
