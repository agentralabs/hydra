---
title: "The Equation"
description: "The differential equation that governs Hydra's state, Lyapunov stability monitoring, and Euler integration."
---

## The Kernel Equation

Hydra's state evolves according to a single differential equation, integrated every 100ms by the ambient thread:

```
dPsi/dt = L-hat Psi + A-hat Psi + G-hat Psi + S-hat Psi - Gamma-hat Psi
```

Each operator contributes a force on Hydra's cognitive state:

| Operator | Name | Weight | Computation |
|----------|------|--------|-------------|
| L-hat | Laplace-Beltrami | 0.30 | `-SUM(xi^2) / dim(Psi)` -- manifold curvature |
| A-hat | Adversarial | 0.25 | `-0.5` if adversarial, else `average_trust` |
| G-hat | Growth | always >= 0 | `log(1 + growth_rate) / 10` |
| S-hat | Signal | variable | `(1 - queue_utilization) x 0.2 - orphan_penalty` |
| Gamma-hat | Dissipation | always > 0 | `base(0.01) + category_penalty(0.05)` |

## Euler Integration

The state is integrated using the forward Euler method with a fixed timestep:

```
V(Psi) = V(Psi_0) + dPsi/dt x dt     where dt = 0.1
```

Every 100ms, the ambient thread computes one step and produces a new **immutable** state snapshot. No mutable global state. Each tick is a fresh measurement.

## Lyapunov Stability

The Lyapunov value `V(Psi)` is the single number that captures Hydra's health:


  
**Stability Levels:**

    | Level | Threshold | Meaning |
    |-------|-----------|---------|
    | Optimal | V >= 0.3 | All systems healthy |
    | Stable | V >= 0.0 | Normal operation |
    | Alert | V >= -0.5 | Intervention may be needed |
    | Critical | V >= -1.0 | Restricted operations |
    | Emergency | V < -1.0 | Emergency protocols activated |
  
  
**Trend Analysis:**

    Trend is computed via linear regression slope over the last 100 values:

    ```
    slope = SUM((xi - x_mean)(vi - v_mean)) / SUM((xi - x_mean)^2)

    Positive slope = improving
    Negative slope = degrading
    ```

    The growth invariant `G-hat >= 0` is constitutionally enforced. If growth turns negative, the kernel raises an error.
  


## What Each Operator Means


#### L-hat: Manifold Curvature

Measures how curved Hydra's cognitive state space is. High curvature means rapid state changes -- the system is navigating complex terrain. Computed from the squared components of the state vector.



#### A-hat: Adversarial Pressure

Reflects the trust field. Under adversarial conditions (prompt injection, data exfiltration attempts), this operator pushes the state toward caution. Under normal conditions, it contributes the average trust score.



#### G-hat: Growth

Capability acquisition rate. Constitutionally constrained to be non-negative -- Hydra's capabilities can only increase. Computed from the logarithm of the growth rate to prevent runaway expansion.



#### S-hat: Signal Health

Communication health across the animus bus. Penalizes orphan signals (messages that were sent but never received) and high queue utilization. Healthy communication contributes positively.



#### Gamma-hat: Dissipation

Entropy and cost. Always positive -- every system has friction. Base dissipation is 0.01, with additional penalty for constitutional violations. This is the force that Hydra must overcome to remain stable.


## The Three Threads

The equation connects the three concurrent threads:

- **Active** thread handles user input, contributing to growth and signal health
- **Ambient** thread integrates the equation and checks invariants every 100ms
- **Dream** thread consolidates beliefs, contributing to manifold curvature changes


:::tip

The equation is not a metaphor. It is Rust code running 10 times per second in `hydra-kernel`, producing immutable state snapshots that determine Hydra's behavior.

:::

