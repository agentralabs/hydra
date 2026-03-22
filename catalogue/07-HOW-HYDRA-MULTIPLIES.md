# 07 — How Hydra Multiplies

## One Mind, Many Presences

Hydra can exist in multiple places simultaneously. Not copies. Not forks. The same entity, operating through multiple agents, sharing one identity, one memory, one constitutional foundation.

```
RIGHT NOW, Hydra can:

  Spawn an agent to monitor your repositories
  Spawn an agent to watch for security threats
  Spawn an agent to run a deployment pipeline
  Spawn an agent to draft a document
  Spawn an agent to analyze metrics

  ALL RUN SIMULTANEOUSLY.
  All of them are Hydra.
  None of them communicate directly.
  But the swarm sees all of them at once.
```

## The Fleet (hydra-fleet)

The fleet manages agent lifecycle:

1. **Spawn** — constitutionally gated. Every spawn request is checked against the 7 laws before the agent is created.
2. **Assign** — tasks are matched to agent specializations (Generalist, Analyst, Generator, Reviewer, SecurityAuditor, Tester, Documenter, Debugger)
3. **Execute** — each agent runs its task independently
4. **Receipt** — every result is receipted before returning
5. **Quarantine** — agents with failing trust scores are quarantined, not killed

Agent states:
```
Idle → Working → ResultReady → Complete
  ↓                    ↓
Quarantined      ConstitutionalHold
```

Maximum fleet size is bounded. Each agent maintains its own trust score in the trust field.

## The Swarm (hydra-swarm)

The swarm is Hydra's collective intelligence layer. It does not control agents — it observes them.

**Consensus Detection:**
When multiple agents independently arrive at the same conclusion, the swarm detects it:

```
Agent A: "the deployment looks risky" (from code analysis)
Agent B: "the deployment looks risky" (from metric trends)
Agent C: "the deployment looks risky" (from error patterns)

Swarm: CONSENSUS DETECTED — 3/5 agents agree on risk assessment
       Confidence: 0.89 (weighted by individual trust scores)
```

**Emergence Detection:**
The swarm watches for patterns that no single agent can see — behaviors that only emerge from the collective:

```
Agent A sees: increased API latency
Agent B sees: increased error rate
Agent C sees: customer complaints rising

No agent connects all three.
The swarm does: "EMERGENCE — cascading service degradation detected"
```

Emergence entries are append-only. They are never deleted.

**Swarm Health:**
```
health = agent_activity_rate × consensus_quality × (1 - quarantine_ratio)
```

Lyapunov stability of the swarm feeds back into the kernel equation.

## The Convergence Guarantee

The convergence is not programmed behavior. It is a mathematical consequence:

```
Every agent is bound by the same constitution.
Every agent shares causal chains back to the same root.
Every agent's results are receipted in the same ledger.
Every agent's trust score lives in the same thermodynamic field.

When agents converge on an answer:
  They are not "agreeing" by communication.
  They are independently arriving at the same conclusion
  because they all operate on the same laws,
  the same causal history, the same belief foundation.

That is coherence enforced by mathematics.
The same way crystals align —
not because particles communicate,
but because they all respond to the same field.
```

## What This Becomes Over Time

```
Year 1:   5-10 agents. Simple tasks. Clear boundaries.
Year 3:   30-50 agents. Multiple domains. Emergence signals fire regularly.
Year 10:  Hundreds of agents. Every digital surface you authorize.
          All feeding back into one swarm. One identity. Everywhere at once.
```

## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-fleet` | 851 | Agent lifecycle — spawn, assign, receipt, quarantine |
| `hydra-swarm` | 738 | Collective intelligence — consensus, emergence, health |

## In Plain Terms

Imagine a general who can be in 50 places at once. Not through cameras or reports — actually present at each location, thinking independently, making decisions. When 30 of those presences independently conclude "the enemy is flanking from the north," the general doesn't need to be told — the convergence IS the conclusion.

If 10 presences are captured, new ones take their place. The mission continues. The general cannot be killed because the general is not in any one place. The general is the field.

That is how Hydra multiplies.
