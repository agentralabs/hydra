# 06 — How Hydra Protects Itself

## The Constitution

Hydra operates under 7 immutable laws. They are compiled into the binary. They cannot be changed at runtime. They are checked on every ambient tick (100ms) and before every state mutation.

| Law | Name | What It Prevents |
|-----|------|-----------------|
| 1 | Receipt Immutability | Receipts cannot be deleted, modified, or forged |
| 2 | Identity Integrity | Identity chain must be cryptographically verifiable |
| 3 | Memory Sovereignty | No external system can overwrite memory without belief revision |
| 4 | Constitution Protection | The constitution itself cannot be modified at runtime |
| 5 | Animus Integrity | Signal bus must maintain causal chain integrity |
| 6 | Principal Supremacy | Human principal always has final authority |
| 7 | Causal Chain Completeness | Every action must trace back to a constitutional root |

Constitutional enforcement happens at **4 write sites** (the highest-risk state mutations):
- **Memory writes** — Law 3 checked before every memory record
- **Belief revisions** — Law 3 checked before modifying the belief manifold
- **Audit records** — Law 1 checked before appending any receipt
- **Identity deepening** — Law 2 checked before extending the hash chain

If any law is violated, the operation is **blocked** — not logged and continued. Blocked.

## Trust Thermodynamics (hydra-trust)

Trust is modeled as a physical quantity using thermodynamic formulas:

```
Each agent has a trust score ∈ [0.0, 1.0]
Score maps to tiers: Bronze (<0.3), Silver (0.3-0.7), Gold (0.7-0.9), Platinum (≥0.9)
Each tier has an energy level: Platinum=0 (lowest), Bronze=5 (highest disorder)

Recovery rate: +0.02 per success
Penalty rate:  -0.05 per failure
Equilibrium:   71% success rate = net zero trust change
```

Trust phase transitions:
- **Stable** (avg ≥ 0.7) — normal operations
- **Elevated** (avg ≥ 0.3) — heightened scrutiny
- **Critical** (avg > 0.0) — restricted operations
- **Collapsed** (avg ≤ 0.0) — emergency protocols

Constitutional violations trigger a **spike**: average trust drops by 0.5 immediately.

## The Immune System (hydra-adversary)

Hydra has an immune system with antibodies:

1. **Threat Signals** arrive with a class (PromptInjection, DataExfiltration, IdentitySpoofing, etc.) and feature vector
2. **Antibodies** match against feature vectors — they recognize threats they have seen before
3. **New threats** generate new antibodies. Antibodies are **never deleted**.
4. **Response**: PassThrough (clean), Blocked (known threat), NewAntibodyGenerated (novel threat detected)

Constitutional threats (attempts to violate the 7 laws) always trigger maximum response.

The antifragile layer records every obstacle encounter. Resistance to each obstacle class **only grows, never decreases**. Hydra gets harder to hurt over time.

## 6 Runtime Invariants

Checked every 100ms by the Ambient thread:

1. **Constitution Reachability** — the checker is always accessible
2. **Animus Bus Health** — queue utilization < 95%, orphans < 100
3. **Lyapunov Stability** — V(Ψ) above alert threshold
4. **Growth Invariant** — growth rate ≥ 0 (capabilities never decrease)
5. **Signal Queue Health** — total drops < 1000
6. **Trust Field Health** — if adversarial conditions exist, average trust ≥ 0.1

If ANY invariant fails → kernel enters degraded mode. The system does not continue as if nothing happened.

## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-constitution` | 2,943 | The 7 laws — immutable, compiled-in |
| `hydra-trust` | 770 | Trust thermodynamics — Hamiltonian phase transitions |
| `hydra-adversary` | 1,048 | Immune system — antibodies, threat ecology |
| `hydra-antifragile` | 382 | Obstacle resistance — only grows, never decreases |

## In Plain Terms

Imagine a fortress with:
- 7 unbreakable rules carved in stone (constitution)
- A health meter that is checked 10 times per second (invariants)
- An immune system that remembers every attack and gets stronger (adversary + antifragile)
- A trust thermometer for every agent that recovers slowly but drops fast (trust)

No one can change the rules. No one can delete the records. No one can erase the memory. Not even Hydra itself. That is constitutional governance.
