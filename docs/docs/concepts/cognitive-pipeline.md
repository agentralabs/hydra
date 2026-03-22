---
title: "The Cognitive Pipeline"
description: "Six stages from comprehension to delivery -- 85% of processing uses zero LLM tokens."
---

## Six Stages of Thought

Every input passes through 6 stages before Hydra responds. Over 85% of cognitive processing is pure structural analysis -- no LLM calls required.

```
Input -> Comprehend -> Route ->  -> Deliver -> Receipt
          Stage 1     Stage 2      Stage 3          Stage 4   Stage 5/6
```



  ### 1. Comprehension

    Domain detection, primitive mapping, memory resonance, confidence scoring. **Zero LLM calls.**
  
  ### 2. Language Analysis

    Intent, affect, depth, and hedge detection run in parallel with comprehension.
  
  ### 3. Context Building

    Five awareness windows combine: active, history, predicted, gap, anomaly.
  
  ### 4. Attention Allocation

    Budget computed from intent and affect. Items scored by urgency, relevance, novelty.
  
  ### 5. Reasoning / LLM

    Five reasoning modes run simultaneously. If synthesis confidence is high enough, zero tokens used.
  
  ### 6. Delivery + Receipt

    Response delivered. Constitutional receipt generated. Memory finalized.
  



## Stage 1: Comprehension

Your raw text is lifted to structured meaning through 4 sub-stages:

1. **Domain Detection** -- match words against domain vocabularies (multiple domains can fire)
2. **Primitive Mapping** -- map concepts to universal axiom primitives via functors
3. **Memory Resonance** -- check the genome store with IDF-weighted scoring
4. **Confidence Score** -- above 0.8 = zero-token resolution possible, below 0.6 = LLM required

## Five Reasoning Modes

When reasoning is needed, five modes run simultaneously:

| Mode | Weight | What It Does |
|------|--------|-------------|
| Deductive | 0.30 | If A implies B and A is true, then B |
| Inductive | 0.25 | Observed 5 times, likely to happen again |
| Abductive | 0.20 | Best explanation for this observation |
| Analogical | 0.15 | Structurally similar to another problem |
| Adversarial | 0.10 | What would go wrong if we did this? |

```
synthesis_confidence = SUM(conclusion_confidence x mode_weight) / SUM(mode_weight)
Threshold: 0.35 minimum for acceptance
```


:::tip

If synthesis confidence exceeds the threshold, the response is generated with **zero LLM tokens**. The 85% zero-token rate comes from genome hits plus high-confidence reasoning synthesis.

:::


## Attention Budget

```
budget = base_for_intent x affect_multiplier

Base:  Analysis=80  Planning=80  Action=50  Status=30
Affect: Crisis=0.5  Neutral=1.0  Exploratory=1.5
```

## The Noticing Engine

Running in the background, always:

> *"I noticed deployment latency increased 3% per week for 6 weeks. Nobody asked me to check."*

The noticing engine watches patterns, detects drift, and surfaces observations nobody requested.

## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-comprehension` | 1,495 | Domain detection, primitive mapping, resonance |
| `hydra-language` | 989 | Intent, affect, depth, hedge detection |
| `hydra-context` | 1,069 | Five-window situational awareness |
| `hydra-attention` | 1,181 | Budget allocation, item scoring |
| `hydra-reasoning` | 1,695 | Five simultaneous reasoning modes |
| `hydra-noticing` | 1,496 | Ambient pattern detection, drift watching |
