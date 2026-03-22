---
title: "The Genome"
description: "Proven approaches that make Hydra smarter with every interaction."
---

## What the Genome Is

The genome is Hydra's DNA — a collection of proven situation→approach pairs with confidence scores and observation counts. When you ask a question, Hydra checks the genome before calling the LLM. If a proven approach exists, it enriches the response.

```toml
[[entries]]
situation    = "service failures cascading to take down other services"
approach     = "install a circuit breaker at every external dependency boundary"
confidence   = 0.92
observations = 5000
```

## IDF-Weighted Retrieval

The genome does not use keyword matching. It uses **IDF-weighted scoring** — rare discriminative terms score higher than common ones:

```
IDF("netflix") = 2.64    ← rare, highly discriminative
IDF("the")     = 0.07    ← common, effectively ignored
```

This means indirect phrasings work: "Netflix failure spreading" matches the circuit breaker entry because "netflix" has high IDF weight.

## Bayesian Confidence

Confidence follows a Beta distribution that updates with real use:

```
Prior:     Beta(α₀, β₀) where α₀ = confidence × 10
After k successes in n uses: Beta(α₀ + k, β₀ + n-k)
Expected:  E[θ] = (α₀ + k) / (α₀ + β₀ + n)
```

## Self-Writing

The dream loop automatically creates genome entries when patterns are detected:

1. Automation engine observes: "deployment safety asked 5 times, circuit breaker worked 4/5"
2. Dream loop checks: observations ≥ 5 AND success ≥ 75%
3. New entry created automatically — no human writes TOML

## 278 Entries Across 28 Skills



  ### Content Creation
38 entries
  ### Developer
30 entries
  ### Finance
26 entries
  ### Architecture
10 entries
  ### Security
10 entries
  ### Physics
10 entries


