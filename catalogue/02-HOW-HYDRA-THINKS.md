# 02 — How Hydra Thinks

## The Cognitive Pipeline

Every input you send passes through 5 stages before Hydra responds. Most of this happens without calling an LLM — over 85% of cognitive processing is pure structural analysis.

```
Input → Comprehend → Route → {Reason or LLM} → Deliver → Receipt
         Stage 1     Stage 2    Stage 3          Stage 4   Stage 5
```

## Stage 1: Comprehension (hydra-comprehension)

Your raw text is lifted to structured meaning through 4 sub-stages:

1. **Domain Detection** — What field is this about? Engineering? Finance? Medicine? Hydra matches your words against domain vocabularies. Multiple domains can fire simultaneously.

2. **Primitive Mapping** — Your words are mapped to universal axiom primitives via functors. "Netflix failure spreading" maps to the Risk primitive. "optimize performance" maps to Volume. These primitives are domain-free — a cascade failure in software and a cascade failure in finance produce the same primitive.

3. **Memory Resonance** — Before processing further, Hydra checks if it has seen something like this before. The genome store is queried with IDF-weighted scoring: rare discriminative terms (like "Netflix") score higher than common terms (like "the"). If a proven approach exists, it surfaces immediately.

4. **Confidence Score** — A number between 0 and 1. How well does Hydra understand what you mean? Above 0.8 = zero-token resolution possible. Above 0.6 = reasoning engine can handle it. Below 0.6 = LLM required.

**Zero LLM calls in this stage.** Pure structural analysis.

## Stage 2: Language Analysis (hydra-language)

Parallel to comprehension, Hydra analyzes:

- **Intent** — What do you want? Execute something? Understand something? Verify something?
- **Affect** — Are you frustrated? Exploratory? Under pressure? This adjusts how much attention budget Hydra allocates.
- **Depth** — Is this a surface question or a deep architectural discussion?
- **Hedge Detection** — Did you say "maybe" or "I think"? Hedges lower Hydra's confidence in the instruction.

## Stage 3: Context Building (hydra-context)

Five windows of awareness combine into a ContextFrame:

1. **Active Window** — What is happening right now
2. **History Window** — What happened in this session
3. **Predicted Window** — What Hydra expects to happen next
4. **Gap Window** — What Hydra doesn't know and should
5. **Anomaly Window** — What is unusual or unexpected

## Stage 4: Attention Allocation (hydra-attention)

Hydra cannot pay equal attention to everything. The attention engine:

1. Scores every context item by urgency, relevance, novelty, and domain match
2. Computes a budget based on intent (analysis gets 80 units, status checks get 30)
3. Multiplies by affect (crisis = narrow focus × 0.5, exploratory = wide focus × 1.5)
4. Allocates items: Full-depth (10 units each), Summary (2 units), or Filtered (0 units)

## Stage 5: Reasoning (hydra-reasoning)

Five reasoning modes run simultaneously:

| Mode | Weight | What It Does |
|------|--------|-------------|
| **Deductive** | 0.30 | If A→B and A is true, then B is true |
| **Inductive** | 0.25 | This happened 5 times, so it will happen again |
| **Abductive** | 0.20 | The best explanation for this observation is... |
| **Analogical** | 0.15 | This is structurally similar to that other problem |
| **Adversarial** | 0.10 | What would go wrong if we did this? |

The first mode to produce a conclusion contributes. All conclusions are synthesized with weighted averaging. If synthesis confidence exceeds the threshold, the response is generated with **zero LLM tokens**.

## Stage 6: Noticing (hydra-noticing)

Running in the background, always:

*"I noticed deployment latency increased 3% per week for 6 weeks. Nobody asked me to check."*

The noticing engine watches patterns, detects drift, and surfaces observations that nobody requested. These appear as ambient signals in the prompt — the LLM sees them alongside your question.

## The Mathematics

**Comprehension confidence:**
```
confidence = domain_match × primitive_count × resonance_boost
```

**Attention budget:**
```
budget = base_for_intent × affect_multiplier
         where crisis=0.5, neutral=1.0, exploratory=1.5
```

**Reasoning synthesis:**
```
synthesis_confidence = Σ(conclusion_confidence × mode_weight) / Σ(mode_weight)
                       where weights = [0.30, 0.25, 0.20, 0.15, 0.10]
```

**IDF-weighted genome retrieval:**
```
score(query, entry) = Σ IDF(term) × 𝟙[term ∈ entry]
IDF(term) = ln((N+1) / (df(term)+1))
```

## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-comprehension` | 1,495 | Domain detection, primitive mapping, resonance |
| `hydra-language` | 989 | Intent, affect, depth, hedge detection |
| `hydra-context` | 1,069 | Five-window situational awareness |
| `hydra-attention` | 1,181 | Budget allocation, item scoring |
| `hydra-reasoning` | 1,695 | Five simultaneous reasoning modes |
| `hydra-noticing` | 1,496 | Ambient pattern detection, drift watching |
| `hydra-learning` | 868 | Observes which modes are accurate, proposes weight changes |
| `hydra-synthesis` | 821 | Cross-domain pattern discovery |

## In Plain Terms

Imagine reading a letter. First you understand the language (comprehension). Then you figure out what they want (intent). Then you recall the context — what happened yesterday, what you expected, what surprised you (context). Then you decide what to focus on (attention). Then you reason from multiple angles simultaneously — logic, experience, analogy, caution (reasoning). And in the background, you notice things nobody pointed out.

That is how Hydra thinks. And 85% of it happens without asking anyone for help.
