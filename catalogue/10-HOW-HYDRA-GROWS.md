# 10 — How Hydra Grows

## The Growth Invariant

Hydra's capabilities can only increase. This is constitutionally enforced — the growth operator Ĝ in the kernel equation is always ≥ 0. If the metabolism monitor detects negative growth (capabilities decreasing), it raises an error.

## Genome (hydra-genome)

The genome is Hydra's DNA — proven situation→approach pairs accumulated over time:

```
Situation: "service failures cascading to take down other services"
Approach:  "install a circuit breaker at every external dependency boundary"
Confidence: 0.92 (Bayesian posterior from 5,000 observations)
```

The genome store is append-only. Entries are never deleted. Confidence updates via Bayesian Beta distribution:

```
Prior:     Beta(α₀, β₀) where α₀ = initial_confidence × 10
After k successes in n uses: Beta(α₀ + k, β₀ + n - k)
Expected:  E[θ] = (α₀ + k) / (α₀ + β₀ + n)
```

New entries can be:
- **Loaded from skill TOML files** (curated knowledge)
- **Crystallized from observed behavior** (automation engine)
- **Transferred from another Hydra** (succession)

## Learning (hydra-learning)

The learning engine is an observer — it never modifies weights directly. It:

1. Watches which reasoning modes (deductive, inductive, abductive, analogical, adversarial) produce accurate conclusions
2. Tracks accuracy per domain: "In security questions, adversarial reasoning is right 85% of the time"
3. Proposes weight adjustments via LearningRecord

```
If accuracy ≥ 0.70: propose increase (up to +0.05 per cycle)
If accuracy ≤ 0.40: propose decrease (down to -0.05 per cycle)
Weights bounded to [0.02, 0.60]
Minimum 5 observations before any adjustment
```

Over the 30-day protocol, reasoning weights adapt per domain.

## Antifragile Resistance (hydra-antifragile)

Hydra grows stronger from obstacles. Every obstacle class (AuthChallenge, RateLimit, NetworkBlock, ProtocolMismatch, etc.) has a resistance record:

```
ObstacleClass: RateLimit
  Encounters: 47
  Wins: 39
  Win rate: 83%
  Resistance: 0.83 (only grows, never decreases)
```

Resistance is the win rate — how often Hydra successfully navigates that obstacle type. It monotonically increases because Hydra learns from each encounter.

## Cartography (hydra-cartography)

Every digital system Hydra encounters is mapped in an append-only atlas:

```
SystemProfile: "production-api"
  Class: RestApi
  Encounters: 156
  Known approaches: ["retry-with-backoff", "circuit-breaker", "fallback-to-cache"]
  Topology hints: ["behind load balancer", "depends on postgres-primary"]
```

Systems are never removed from the atlas. Knowledge only accumulates.

## Plasticity (hydra-plastic)

Hydra adapts its execution strategy based on accumulated experience:

```
Environment: "loop-llm-short"
  Mode: NativeBinary
  Encounters: 342
  Success rate: 94%
  Confidence: 0.91
```

The plasticity tensor tracks which execution modes work best in which environments. It is append-only.

## Generative Capability (hydra-generative)

Hydra can synthesize new capabilities from axiom primitives:

1. **Decompose** a task into subtasks
2. **Check** which subtasks match existing capabilities
3. **Detect gaps** — capabilities that don't exist yet
4. **Compose** new capabilities from existing primitives
5. **Estimate confidence** in the synthesized capability

The capability ceiling is mathematical infinity — any combination of primitives can be composed.

## Skills (hydra-skills)

Skills are hot-loadable capability packages:

```
skills/general/
  genome.toml     — situation/approach pairs
  functor.toml    — domain concept → axiom primitive mappings
```

Every skill load is constitutionally gated. Knowledge persists in the genome even after a skill is unloaded.

## Soul (hydra-soul)

The orientation layer. Hydra knows what the work is for — not just what the work is.

```
The MeaningGraph accumulates meaning from every exchange.
After enough data, Soul provides orientation context:
  "Your work on deployment safety connects to your broader goal
   of reliable infrastructure. This is a recurring theme across
   47 sessions over 3 months."
```

Soul node weights decay exponentially:
```
w(t) = w₀ × e^(-λt)    where λ = 0.0001 per day
Half-life ≈ 19 years
```

Old orientations fade slowly but never vanish. They can be reinforced if they become relevant again.

## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-genome` | 961 | Proven approaches — IDF-scored, Bayesian confidence |
| `hydra-learning` | 868 | Reasoning mode accuracy tracking |
| `hydra-antifragile` | 382 | Obstacle resistance (only grows) |
| `hydra-cartography` | 631 | System mapping (append-only atlas) |
| `hydra-plastic` | 612 | Environment adaptation (append-only tensor) |
| `hydra-generative` | 586 | Capability composition from primitives |
| `hydra-skills` | 566 | Hot-loadable skill substrate |
| `hydra-soul` | 1,382 | Orientation — what the work is for |
| `hydra-reflexive` | 673 | Self-model — runtime capability map |

## In Plain Terms

Imagine someone who:
- Remembers every successful approach and reuses it (genome)
- Tracks which thinking styles work best for which problems (learning)
- Gets literally stronger every time they face a challenge (antifragile)
- Maps every system they touch and never forgets the layout (cartography)
- Adapts their work style to each environment automatically (plasticity)
- Can combine existing skills to create new capabilities on the fly (generative)
- Knows not just WHAT they are doing but WHY it matters in the bigger picture (soul)

That is how Hydra grows. And none of it ever decreases.
