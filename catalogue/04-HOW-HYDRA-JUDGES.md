# 04 — How Hydra Judges

## The Difference Between Intelligence and Judgment

Intelligence is knowing the answer. Judgment is knowing when the answer might be wrong.

*"The data says X. But pattern history says be careful. Three times before when data said X, it was wrong because of Y."*

That is judgment. That is what Layer 4 produces.

## Pattern Library (hydra-pattern)

Hydra maintains a library of domain-agnostic patterns — both success patterns and anti-patterns. A cascade failure in engineering is structurally identical to a volatility cascade in finance. The pattern library recognizes both.

When your input produces axiom primitives, the pattern engine checks for warnings:
- *"This combination of Risk + Dependency primitives matches the 'cascade failure' anti-pattern (observed 47 times, 89% led to outage)"*
- *"This matches the 'interface-first' success pattern (92% success rate across 4,000 observations)"*

## Red Team (hydra-redteam)

**Before** Hydra acts, it simulates what an intelligent attacker would do:

1. **Identify attack surfaces** from the context
2. **Generate threat vectors** from the axiom primitives
3. **Score each threat** by severity and risk
4. **Produce a verdict**: Go / Go-with-mitigations / No-Go

No-Go is not a failure — it is judgment preventing a mistake.

## Calibration (hydra-calibration)

Hydra tracks its own confidence errors:

```
"My raw confidence is 0.83.
 I have a known +0.11 overconfidence bias in this domain.
 Calibrated confidence: 0.72."
```

The calibration engine:
1. Records every prediction with its stated confidence
2. Later records the actual outcome
3. Computes bias per domain and judgment type (mean offset, standard deviation)
4. Applies correction: `calibrated = clamp(raw + correction, 0, 1)`

Significance requires ≥10 samples and offset ≥0.05. Below that, Hydra reports "unreliable — insufficient calibration data."

### The Bayesian Foundation

Confidence follows a Beta distribution:
```
Prior:     Beta(α₀, β₀) where α₀ = confidence × 10
Posterior: Beta(α₀ + successes, β₀ + failures)
Expected:  E[θ] = α / (α + β)
```

With 0 uses, the posterior equals the prior. With many uses, it converges to the true success rate. This is mathematically optimal for early-data handling.

## Oracle (hydra-oracle)

Probabilistic future modeling:
1. Takes axiom primitives from comprehension
2. Maps each primitive type to a scenario archetype
3. Assigns probabilities to each scenario
4. Identifies adverse outcomes
5. Suggests interventions for the worst cases

*"3 scenarios projected. 1 adverse (probability 0.35). Intervention: add circuit breaker before deployment."*

## Omniscience (hydra-omniscience)

Every other AI: *"I don't know."*
Hydra: *"I don't know yet. Acquiring."*

Gap detection → acquisition plan → multiple sources → belief integration → gap closed. Permanently.

The omniscience engine tracks knowledge gaps:
- **Factual** — missing knowledge about a concept
- **Procedural** — missing knowledge about how to do something
- **Contextual** — missing knowledge about current state
- **Structural** — missing knowledge about how things connect

Gaps are tracked with priority and recurrence count. Recurring gaps get escalated.

## Wisdom (hydra-wisdom)

Where intelligence becomes judgment. Layer 4 closes here.

The wisdom engine synthesizes evidence from all other Layer 4 crates:
- Pattern warnings → PatternEvidence
- Oracle projections → OracleEvidence
- Red team analysis → RedTeamEvidence
- Calibration data → CalibrationEvidence

And produces a `WisdomStatement`:
- **Recommendation**: Proceed / ProceedWithConditions / PauseAndVerify / DoNotProceed
- **Confidence**: calibrated
- **Key Uncertainties**: what could change this judgment
- **Reversal Conditions**: what would make the opposite recommendation correct

## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-pattern` | 762 | Domain-agnostic pattern library |
| `hydra-redteam` | 943 | Proactive adversarial simulation |
| `hydra-calibration` | 1,217 | Epistemic calibration — knows where it goes wrong |
| `hydra-oracle` | 535 | Probabilistic scenario projection |
| `hydra-omniscience` | 1,354 | Active knowledge gap detection and closure |
| `hydra-wisdom` | 1,332 | Evidence synthesis → judgment |

## In Plain Terms

Imagine a doctor who:
- Remembers every misdiagnosis they ever made and adjusts for bias (calibration)
- Before prescribing, imagines what an adversary would exploit in the treatment (red team)
- Checks whether this patient's symptoms match known disease patterns (pattern library)
- Considers what could go wrong in the next 6 months (oracle)
- Admits what they don't know and actively researches it (omniscience)
- Combines all of this into one recommendation with explicit uncertainties (wisdom)

That is how Hydra judges.
