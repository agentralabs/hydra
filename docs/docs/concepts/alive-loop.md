---
title: "The Alive Loop"
description: "Three concurrent threads that make Hydra a persistent entity."
---

## Three Threads, Never Stopping

Hydra is not a request-response system. It is three concurrent loops running continuously:



  ### ACTIVE

    **Priority 9** — Responds to your input through the full cognitive pipeline.
  
  ### AMBIENT

    **Every 100ms** — Integrates the kernel equation, checks 6 invariants, dispatches signals.
  
  ### DREAM

    **Every 500ms** — Consolidates beliefs, discovers patterns, writes genome entries from experience.
  



## The Kernel Equation

Every 100ms, the ambient thread computes:

```
dΨ/dt = L̂Ψ + ÂΨ + ĜΨ + ŜΨ − Γ̂Ψ

L̂ = manifold curvature (cognitive state space)
 = trust field pressure (adversarial detection)
Ĝ = growth rate (capability acquisition, always ≥ 0)
Ŝ = signal health (communication between subsystems)
Γ̂ = dissipation (entropy + cost, always positive)
```

The Lyapunov value `V(Ψ)` is the single number that captures Hydra's health:

| Value | Status | Meaning |
|-------|--------|---------|
| ≥ 0.3 | Optimal | All systems nominal |
| ≥ 0.0 | Stable | Acceptable, minor concerns |
| ≥ -0.5 | Alert | Intervention level 1 |
| ≥ -1.0 | Critical | Intervention level 2 |
| < -1.0 | Emergency | Intervention level 3 |
