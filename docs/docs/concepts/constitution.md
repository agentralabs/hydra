---
title: "The Constitution"
description: "Seven immutable laws, constitutional receipting at 4 write sites, and runtime invariant checks."
---

## Seven Immutable Laws

Hydra operates under 7 laws compiled into the binary. They cannot be changed at runtime. They are checked on every ambient tick (100ms) and before every state mutation.

| Law | Name | What It Prevents |
|-----|------|-----------------|
| 1 | Receipt Immutability | Receipts cannot be deleted, modified, or forged |
| 2 | Identity Integrity | Identity chain must be cryptographically verifiable |
| 3 | Memory Sovereignty | No external system can overwrite memory without belief revision |
| 4 | Constitution Protection | The constitution itself cannot be modified at runtime |
| 5 | Animus Integrity | Signal bus must maintain causal chain integrity |
| 6 | Principal Supremacy | Human principal always has final authority |
| 7 | Causal Chain Completeness | Every action must trace back to a constitutional root |


:::warning

If any law is violated, the operation is **blocked** -- not logged and continued. Blocked. No exceptions.

:::


## Constitutional Receipting at 4 Write Sites

Enforcement happens at the 4 highest-risk state mutations:



  ### Memory Writes

    Law 3 checked before every memory record to protect memory sovereignty.
  
  ### Belief Revisions

    Law 3 checked before modifying the belief manifold.
  
  ### Audit Records

    Law 1 checked before appending any receipt to ensure immutability.
  
  ### Identity Deepening

    Law 2 checked before extending the morphic hash chain.
  



## Six Runtime Invariants

Checked every 100ms by the Ambient thread:

1. **Constitution Reachability** -- the checker is always accessible
2. **Animus Bus Health** -- queue utilization < 95%, orphans < 100
3. **Lyapunov Stability** -- V(Psi) above alert threshold
4. **Growth Invariant** -- growth rate >= 0 (capabilities never decrease)
5. **Signal Queue Health** -- total drops < 1000
6. **Trust Field Health** -- if adversarial conditions exist, average trust >= 0.1


:::tip

If ANY invariant fails, the kernel enters degraded mode. The system does not continue as if nothing happened.

:::


## Trust Thermodynamics

Trust is modeled as a physical quantity:

```
Score:     s in [0.0, 1.0]
Tiers:     Bronze (&lt;0.3), Silver (0.3-0.7), Gold (0.7-0.9), Platinum (>=0.9)
Recovery:  +0.02 per success
Penalty:   -0.05 per failure
Spike:     constitutional violation -> average trust drops by 0.50

Equilibrium: 71% success rate = net zero trust change
```

## The Immune System

Hydra has an antibody-based immune system that gets stronger from every attack:


#### How the immune system works

1. **Threat signals** arrive with a class and feature vector
2. **Antibodies** match against known threats
3. **Novel threats** generate new antibodies (antibodies are never deleted)
4. **Antifragile resistance** to each obstacle class only grows, never decreases

Constitutional threats always trigger maximum response.


## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-constitution` | 2,943 | The 7 laws -- immutable, compiled-in |
| `hydra-trust` | 770 | Trust thermodynamics with phase transitions |
| `hydra-adversary` | 1,048 | Immune system with antibody memory |
| `hydra-antifragile` | 382 | Obstacle resistance -- only grows |
