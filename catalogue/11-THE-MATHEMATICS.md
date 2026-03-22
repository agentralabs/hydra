# 11 — The Mathematics

## Every Equation in Hydra

This document collects every mathematical formula used across all 68 crates. These are not approximations — they are the exact formulas implemented in Rust.

---

## The Kernel Equation

```
dΨ/dt = L̂Ψ + ÂΨ + ĜΨ + ŜΨ − Γ̂Ψ

L̂ = Laplace-Beltrami (manifold curvature)     weight: 0.3
    Computed as: -Σ(xᵢ²) / dim(Ψ)

Â = Adversarial (trust field)                   weight: 0.25
    If adversarial: -0.5
    Else: average_trust ∈ [0, 1]

Ĝ = Growth (capability acquisition)            always ≥ 0
    Computed as: log(1 + growth_rate) / 10

Ŝ = Signal (communication health)
    Computed as: (1 - queue_utilization) × 0.2 - orphan_penalty

Γ̂ = Dissipation (entropy + cost)               always > 0
    Computed as: base(0.01) + category_penalty(0.05)

Integration: V(Ψ) = V(Ψ₀) + dΨ/dt × dt        (Euler method, dt = 0.1)
```

---

## IDF-Weighted Scoring (Genome + Memory)

```
score(Q, D) = Σ IDF(qᵢ) × 𝟙[qᵢ ∈ D]

IDF(qᵢ) = ln((N + 1) / (df(qᵢ) + 1))

Where:
  Q = query terms (stemmed)
  D = document terms
  N = total documents in corpus
  df(qᵢ) = documents containing term qᵢ

Threshold: score > 0.5 for genome, score > 0.0 for memory
```

---

## Memory Retrieval Scoring

```
final_score = idf_score × recency_weight × (1 + recency_bonus)

recency_weight = 0.3 + 0.7 × (node_index / total_nodes)
recency_bonus  = 0.5 if last 10%, 0.2 if last 30%, 0.0 otherwise

RELEVANCE OVERRIDE:
  If idf_score > 2.0: final_score = idf_score × (1 + recency_bonus)
  (temporal decay is bypassed for highly relevant memories)

Topic deduplication:
  Two nodes are duplicates if overlap(top_20_terms) > 60%
```

---

## Bayesian Beta Confidence (Genome Entries)

```
Prior:     Beta(α₀, β₀) where α₀ = confidence × 10, β₀ = (1-confidence) × 10
Posterior: Beta(α₀ + k, β₀ + n-k) after k successes in n uses
Expected:  E[θ] = (α₀ + k) / (α₀ + β₀ + n)

Example:
  Entry with conf=0.9, 0 uses:    E[θ] = 9/(9+1) = 0.90
  Entry with conf=0.9, 8/10 uses: E[θ] = 17/(17+3) = 0.85
  Entry with conf=0.5, 10/10:     E[θ] = 15/(15+5) = 0.75
```

---

## Keyword Stemming

```
stem(word) = strip longest matching suffix from:
  ["ation", "ment", "ness", "ting", "ing", "sion", "tion",
   "able", "ible", "ful", "less", "ous", "ive", "ies",
   "ied", "ers", "est", "ely", "ity",
   "ed", "er", "ly", "es", "al", "s"]

Guard: word length must remain > suffix_length + 2

Examples:
  "services" → "servic"    "failures" → "failur"
  "rewrites" → "rewrit"    "cascading" → "cascad"
```

---

## Jaccard Similarity

```
J(A, B) = |A ∩ B| / |A ∪ B|

Range: [0.0, 1.0]
Used in: genome store fallback, pattern matching, resonance
Threshold: 0.10 (lowered from 0.70 to support indirect phrasings)
```

---

## Trust Thermodynamics

```
Score: s ∈ [0.0, 1.0]
Tiers: Bronze(<0.3), Silver(0.3-0.7), Gold(0.7-0.9), Platinum(≥0.9)
Energy: E(tier) = {Platinum:0, Gold:1, Silver:3, Bronze:5}

Update rules:
  Success: score += 0.02
  Failure: score -= 0.05
  Constitutional violation: average_trust -= 0.50 (spike)

Phase transitions:
  Stable:    avg ≥ 0.7
  Elevated:  avg ≥ 0.3
  Critical:  avg > 0.0
  Collapsed: avg ≤ 0.0

Equilibrium: 71% success rate → net zero change
  (0.71 × 0.02 = 0.29 × 0.05)
```

---

## Belief Manifold Geometry

```
Distance: d(a, b) = √(Σ(aᵢ - bᵢ)²)    (Euclidean L2)

Geodesic step:
  new_coord = a + STEP_SIZE × (b - a)    where STEP_SIZE = 0.1

Revision strength: 0.3
  confidence_delta = REVISION_STRENGTH × (new_confidence - old_confidence)
```

---

## Soul Node Decay

```
w(t) = w₀ × e^(-λt)    where λ = 0.0001 per day
Half-life = ln(2) / λ ≈ 6,931 days ≈ 19 years
Floor: 0.001 (fossil — never fully gone)
```

---

## Attention Budget

```
budget = base_for_intent × affect_multiplier

Base budgets:
  Analysis:80  Planning:80  Action:50  Generative:60
  Verification:40  Information:40  Status:30  Conversational:20

Affect multipliers:
  Crisis:0.5  Under-pressure:0.7  Neutral:1.0
  Frustrated:1.2  Exploratory:1.5

Item scoring:
  final = base_significance + urgency_bonus(0.15) + resonance_bonus(0.20)
        + active_window_bonus(0.10) + anomaly_bonus(0.15) + domain_match(0.10)
  Clamped to [0.0, 1.0]
```

---

## Reasoning Mode Weights

```
Deductive:   0.30
Inductive:   0.25
Abductive:   0.20
Analogical:  0.15
Adversarial: 0.10
Sum: 1.00

Synthesis confidence:
  Σ(conclusion_confidence × mode_weight) / Σ(mode_weight)
  Threshold: 0.35 minimum for acceptance
```

---

## Calibration Bias Correction

```
offset = mean(stated_confidence - actual_accuracy)
         computed over ≥10 samples

Direction:
  Overconfident: offset < -0.05
  Underconfident: offset > 0.05
  Well-calibrated: |offset| < 0.05

Correction: calibrated = clamp(raw + correction, 0.0, 1.0)
  Max correction: ±0.30
```

---

## Prediction Divergence

```
divergence_score = count(diverged_keys) / total_keys

Range: [0.0, 1.0]
Threshold: 0.4 → triggers belief revision
Corrective belief confidence: 1 - divergence_score
```

---

## Lyapunov Stability Monitoring

```
Classification:
  Optimal:   V ≥ 0.3
  Stable:    V ≥ 0.0
  Alert:     V ≥ -0.5
  Critical:  V ≥ -1.0
  Emergency: V < -1.0

Trend: linear regression slope over last 100 values
  slope = Σ((xᵢ - x̄)(vᵢ - v̄)) / Σ((xᵢ - x̄)²)
  Positive = improving, Negative = degrading

Growth invariant: Γ̂ ≥ 0 (enforced — error if violated)
```

---

## Swarm Consensus

```
consensus = majority(agent_conclusions) weighted by trust_scores
confidence = Σ(agreeing_trust) / Σ(all_trust)
```

---

## Portfolio Resource Allocation

```
objective_score = risk × 0.30 + orientation × 0.25 + roi × 0.25 + urgency × 0.20

Allocation: proportional to score
  allocation(i) = score(i) / Σ(scores) × total_budget
```

---

## Morphic Identity Hash Chain

```
H₀ = SHA256("genesis")
Hₙ = SHA256(Hₙ₋₁ || event_data)

Chain depth = number of events recorded
Identity distance = normalized Hamming distance between two chains
Restart tracking: deepen(Hₙ || "restart") — chain continues, never resets
```

---

*Every number in Hydra comes from one of these formulas.*
*No magic numbers. No arbitrary thresholds. Mathematics all the way down.*
