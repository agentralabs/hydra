# 08 — How Hydras Communicate

## Sovereign Peers

When two Hydra instances discover each other, neither is in charge. Federation connects peers without subordinating either. Every sharing decision is:
- **Specific** — exactly what data, for what purpose
- **Consented** — both sides agree before anything transfers
- **Receipted** — every shared item has an immutable audit record
- **Revocable** — consent can be withdrawn at any time

## Federation (hydra-federation)

The federation protocol:

```
1. DISCOVER  — Hydra A finds Hydra B on the network
2. VERIFY    — cryptographic identity check (morphic signature)
3. PROPOSE   — Hydra A offers a sharing scope: "I'll share pattern data if you share calibration data"
4. NEGOTIATE — Hydra B counter-offers or accepts
5. SESSION   — an active sharing session begins with agreed scope
6. SHARE     — data flows within scope boundaries only
7. REVOKE    — either side can end the session at any time
```

Every sharing event references the consent that authorized it.

## Consensus (hydra-consensus)

When two Hydras hold conflicting beliefs:

```
Hydra A: "Deployment X is safe" (confidence 0.8, based on 15 observations)
Hydra B: "Deployment X is risky" (confidence 0.7, based on 8 observations)
```

Neither overwrites the other. The consensus engine applies AGM belief revision extended to two agents:

1. Compare evidence quality (number of observations, recency)
2. Compare calibration reliability (are these confidence numbers trustworthy?)
3. Produce a merged belief with provenance from both
4. If uncertain, mark as uncertain — never force resolution

The merged belief carries attribution: *"Based on 23 combined observations from 2 instances."*

## Consent (hydra-consent)

Fine-grained sharing consent:

```rust
ConsentGrant {
    peer_id: "hydra-B",
    scope: ConsentScope::PatternData,
    max_uses: Some(100),
    valid_days: 30,
}
```

- No consent → no sharing. Hard stop.
- Expired consent → sharing blocked. No grace period.
- Every use is counted toward `max_uses`.
- Every sharing event is recorded in the consent audit log.

## Collective Intelligence (hydra-collective)

```
One Hydra sees a pattern 3 times.
Ten federated Hydras see it 47 times combined.
The pattern is now proven, not suspected.
```

Collective intelligence is:
- **Peer-to-peer** — no central registry
- **Trust-weighted** — observations from high-trust peers count more
- **Consent-gated** — only peers with active consent contribute

## Diplomacy (hydra-diplomat)

When multiple Hydra instances need to coordinate on a decision:

1. A diplomacy session opens on a topic
2. Each instance submits a **stance** — its position with supporting evidence
3. Stances are synthesized into a **joint recommendation**
4. Minority positions are **preserved, never suppressed**
5. Disagreement is a signal, not a bug

No participant is "in charge." The recommendation emerges from synthesis.

## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-federation` | 1,570 | Peer discovery, trust negotiation, session management |
| `hydra-consensus` | 497 | Cross-instance belief resolution (AGM for two agents) |
| `hydra-consent` | 1,097 | Fine-grained, time-bounded, revocable sharing consent |
| `hydra-collective` | 471 | Distributed pattern intelligence — P2P, trust-weighted |
| `hydra-diplomat` | 1,226 | Multi-instance coordination — no leader, minority preserved |

## In Plain Terms

Imagine two experts who each know different things. They meet, verify each other's credentials, agree on what to share, share within strict boundaries, and produce combined insights that neither could have alone. If they disagree, neither is forced to change their mind — the disagreement is recorded as valuable information.

Now imagine 100 experts doing this simultaneously. That is Hydra federation.
