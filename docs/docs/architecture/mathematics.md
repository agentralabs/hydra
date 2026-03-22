---
title: "Mathematics"
description: "Every equation in Hydra -- IDF scoring, Bayesian Beta, Lyapunov stability, exponential decay, and more."
---

## Every Number Has a Formula

No magic numbers. No arbitrary thresholds. Every computation in Hydra's 68 crates traces to one of these equations.

## IDF-Weighted Scoring

Used in genome retrieval and memory search:

```
score(Q, D) = SUM( IDF(qi) x 1[qi in D] )

IDF(qi) = ln((N + 1) / (df(qi) + 1))

N  = total documents in corpus
df = documents containing term qi

Threshold: > 0.5 for genome, > 0.0 for memory
```


#### Example

```
IDF("circuit") = ln(150/3) = 3.91   <- rare, discriminative
IDF("the")     = ln(150/140) = 0.07 <- common, ignored
```
Rare terms dominate. Common terms contribute almost nothing. This is how human memory works -- you remember what is distinctive.


## Bayesian Beta Confidence

Genome entry confidence updates with real-world evidence:

```
Prior:     Beta(a0, b0)  where a0 = confidence x 10, b0 = (1-confidence) x 10
Posterior: Beta(a0 + k, b0 + n-k)  after k successes in n uses
Expected:  E[theta] = (a0 + k) / (a0 + b0 + n)
```

| Scenario | E[theta] |
|----------|----------|
| conf=0.9, 0 uses | 0.90 |
| conf=0.9, 8/10 successes | 0.85 |
| conf=0.5, 10/10 successes | 0.75 |

## Memory Retrieval

```
final_score = idf_score x recency_weight x (1 + recency_bonus)

recency_weight = 0.3 + 0.7 x (node_index / total_nodes)
recency_bonus  = 0.5 if last 10%, 0.2 if last 30%, 0.0 otherwise

RELEVANCE OVERRIDE:
  If idf_score > 2.0: bypass temporal decay entirely
```

## The Kernel Equation

```
dPsi/dt = L-hat(0.3) + A-hat(0.25) + G-hat(>=0) + S-hat - Gamma-hat(>0)

L-hat = -SUM(xi^2) / dim(Psi)         manifold curvature
A-hat = -0.5 if adversarial, else avg_trust
G-hat = log(1 + growth_rate) / 10      always non-negative
S-hat = (1 - queue_util) x 0.2 - orphan_penalty
Gamma = 0.01 + category_penalty(0.05)  always positive

Integration: V(Psi) = V(Psi_0) + dPsi/dt x 0.1   (Euler, dt=0.1)
```

## Lyapunov Stability

```
Optimal:   V >= 0.3
Stable:    V >= 0.0
Alert:     V >= -0.5
Critical:  V >= -1.0
Emergency: V < -1.0

Trend: linear regression slope over last 100 values
```

## Trust Thermodynamics

```
Success: score += 0.02
Failure: score -= 0.05
Constitutional violation: avg_trust -= 0.50

Equilibrium: 71% success rate (0.71 x 0.02 = 0.29 x 0.05)
Tiers: Bronze(&lt;0.3), Silver(0.3-0.7), Gold(0.7-0.9), Platinum(>=0.9)
```

## Exponential Decay (Soul Nodes)

```
w(t) = w0 x e^(-lambda x t)    lambda = 0.0001 per day
Half-life = ln(2) / lambda = 6,931 days = 19 years
Floor: 0.001 (fossil -- never fully gone)
```

## Additional Equations


  
**Attention Budget:**

    ```
    budget = base_for_intent x affect_multiplier
    Base: Analysis=80, Planning=80, Action=50, Status=30
    Affect: Crisis=0.5, Neutral=1.0, Exploratory=1.5
    ```
  
  
**Reasoning Synthesis:**

    ```
    confidence = SUM(conclusion_conf x mode_weight) / SUM(mode_weight)
    Weights: Deductive=0.30, Inductive=0.25, Abductive=0.20,
             Analogical=0.15, Adversarial=0.10
    Threshold: 0.35 minimum
    ```
  
  
**Calibration:**

    ```
    offset = mean(stated_confidence - actual_accuracy)
    Overconfident:    offset < -0.05
    Underconfident:   offset > 0.05
    Well-calibrated: |offset| < 0.05
    Max correction: +/- 0.30
    ```
  
  
**Identity Hash Chain:**

    ```
    H0 = SHA256("genesis")
    Hn = SHA256(H(n-1) || event_data)
    Restart: deepen(Hn || "restart") -- chain continues, never resets
    ```
  

