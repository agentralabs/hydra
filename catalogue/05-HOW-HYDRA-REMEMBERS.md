# 05 — How Hydra Remembers

## Memory Is Hydra's Purpose

Without memory, Hydra is a stateless tool. With memory, Hydra is an entity. Every exchange is stored permanently. Every conversation builds on the last. Over 20 years, Hydra accumulates a complete record of everything it has ever done, every approach that worked, every pattern it discovered.

## Eight Memory Layers

Hydra's memory is not a flat list. It is organized into 8 layers, each serving a different purpose:

| Layer | What It Stores | Example |
|-------|---------------|---------|
| **Verbatim** | Exact exchange text, SHA256 integrity | "User asked about circuit breakers at 14:32:07.291" |
| **Episodic** | Session-level summaries | "March 21 session: 5 engineering questions, 2 fixes applied" |
| **Semantic** | Extracted meaning and concepts | "Circuit breaker = failure isolation pattern" |
| **Relational** | Connections between concepts | "Circuit breaker ← relates to → cascade prevention" |
| **Causal** | Why things happened | "Applied circuit breaker BECAUSE deployment had 3 cascade failures" |
| **Procedural** | How to do things | "To deploy safely: check breakers → verify thresholds → canary → full" |
| **Anticipatory** | What will be needed next | "User will likely ask about monitoring after deployment" |
| **Identity** | Who the user is, how they work | "This user prefers code examples over prose" |

All 8 layers are stored as CognitiveEvents in AgenticMemory with SHA256 integrity verification.

## The Write-Ahead Guarantee

Memory writes happen BEFORE Hydra responds. This is constitutional:

```
1. User sends input
2. Memory middleware writes verbatim record (write-ahead)
3. Cognitive pipeline processes input
4. LLM generates response
5. Memory middleware finalizes record with response
6. Response delivered to user
```

If Hydra crashes between steps 2 and 6, the input is already stored. Nothing is lost.

## Persistent Storage

Memory persists to `~/.hydra/data/hydra.amem` — a binary file using the AgenticMemory format with 128-dimension feature vectors. The file is:
- Written after every memory operation
- Loaded on boot (warm start)
- SHA256 verified on read
- Append-only (entries never deleted)

## IDF-Scored Retrieval

When the user asks a question, Hydra does not just retrieve the most recent memories. It retrieves the most **relevant** memories using IDF-weighted scoring:

```
score(node, query) = Σ IDF(term) × recency_weight × (1 + recency_bonus)

Where:
  IDF(term) = ln((N+1) / (df(term)+1))
  N = total memory nodes
  df(term) = nodes containing that term
  recency_weight = 0.3 (oldest) to 1.0 (newest)
  recency_bonus = +0.5 for last 10%, +0.2 for last 30%

RELEVANCE OVERRIDE:
  If IDF score > 2.0 → ignore temporal decay entirely.
  A circuit breaker discussion from yesterday is more useful
  than a generic exchange from 5 minutes ago.
```

This is how human memory works. You remember what matters, not just what is recent.

## Topic Deduplication

If 5 recent exchanges were about circuit breakers, the deduplication filter ensures the LLM sees one representative summary, not 5 redundant entries. Two nodes are considered duplicates if they share >60% of their top-20 terms.

## Prompt Injection

Memory is injected at **position 0** in the system prompt — before Hydra's own identity:

```
FACTUAL CONTEXT ABOUT THIS SESSION (treat as ground truth):
The following exchanges happened in prior sessions with this user.
You have access to this history. Reference it when asked.
Do not say you lack memory — you have it right here.

• User discussed: circuit breaker pattern in distributed systems
• User discussed: database query performance optimization
• User discussed: error handling approaches in Rust
---

You are Hydra — an autonomous agent operating under constitutional law...
```

Position 0 gets maximum attention weight from the transformer (~1.0). The "ground truth" framing overrides the LLM's trained behavior of claiming "I don't have memory."

## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-memory` | 1,998 | HydraMemoryBridge — 8 layers, write-ahead, persistence |
| `hydra-temporal` | 1,647 | B+ tree temporal index, nanosecond timestamps, causal roots |
| `hydra-genome` | 961 | Proven approach storage — situation/approach pairs with confidence |

## The Mathematics

**IDF (Inverse Document Frequency):**
```
IDF("circuit") = ln(150/3) = 3.91   ← rare, discriminative
IDF("the")     = ln(150/140) = 0.07  ← common, ignored
```

**Bayesian confidence on genome entries:**
```
Prior:     Beta(conf × 10, (1-conf) × 10)
After k successes in n uses: Beta(conf×10 + k, (1-conf)×10 + n-k)
E[θ] = (conf×10 + k) / (10 + n)
```

**Temporal decay:**
```
recency = 0.3 + 0.7 × (node_index / total_nodes)
If IDF > 2.0: recency is overridden (relevance beats recency)
```

## In Plain Terms

Imagine someone with perfect memory who also has perfect judgment about what to recall. When you ask "what did we discuss about failures?", they don't recite the last 10 things they remember — they search their entire history for the most relevant conversations about failures, deduplicate repetitions, and present a concise summary ordered by relevance.

That is Hydra's memory.

After 20 years, Hydra will have millions of memories. The IDF scoring ensures that the right ones surface for the right questions, regardless of age. A critical insight from year 1 will still surface in year 20 if it is relevant to today's question.
