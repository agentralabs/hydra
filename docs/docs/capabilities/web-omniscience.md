---
title: "Web Omniscience"
description: "Three-layer knowledge resolution: genome, index, search. 83 sources. The internet internalized."
---

## Three Layers of Knowledge

Hydra does not store the internet. It internalizes it. Each question resolves through three layers, and the answer drops down to layer 1 for every future query.



  ### Layer 1: Genome

    **Zero web calls.** Answer already internalized from a previous lookup, a skill file, or self-written entry. 90% of queries end here after month 6.
  
  ### Layer 2: Index

    **One targeted call.** Hydra knows WHERE to look. Direct hit to the right source. Result stored in genome for next time.
  
  ### Layer 3: Search

    **One broad call.** Topic is unknown. Search the web once. Index the source. Crystallize the answer. Never search for this again.
  



## The Internalization Cycle

```
Day 1:     20 questions -> 15 web calls
Week 1:    20 questions ->  8 web calls
Month 1:   20 questions ->  4 web calls
Month 6:   20 questions ->  1 web call
Year 1:    20 questions ->  0 web calls most days
```


:::tip

This is not caching. Caching stores responses. Hydra stores **understanding**. A genome entry for "circuit breaker" is a proven approach with confidence scores and Bayesian updating, not a cached web page.

:::


## Seeded Knowledge Sources

Hydra ships with 21 foundational sources across 6 categories:


  
**Programming:**

    Rust (doc.rust-lang.org), Python (docs.python.org), JavaScript (MDN), TypeScript, Go
  
  
**Infrastructure:**

    Kubernetes, Docker, Terraform
  
  
**Architecture:**

    Martin Fowler (circuit breaker), microservices.io, Refactoring Guru
  
  
**Science:**

    Physics, Chemistry, Biology, Mathematics (Wikipedia)
  
  
**Finance:**

    Markets (Investopedia), Crypto (Wikipedia)
  
  
**AI/ML:**

    scikit-learn, Wikipedia (neural networks), Hugging Face (transformers)
  


These grow automatically. Every successful lookup adds new sources to the index.

## Web Integrations

Three web integrations ship ready to use:

| Integration | Capabilities |
|-------------|-------------|
| **Brave Search** | Web search, news search |
| **GitHub Knowledge** | Search repos, search code, read READMEs, read files, issues, commits |
| **Wikipedia** | Article summaries, full content, topic search (no API key needed) |

## The Dream Loop Explores

While you sleep, the dream loop notices topics you asked about and proactively indexes sources:

```
Today: you asked about gRPC 3 times
Tonight: dream loop queues grpc.io for exploration
  -> Indexes as a source (reliability 96%)
  -> Crystallizes key concepts as genome entries
Tomorrow: "How does gRPC handle streaming?"
  -> Genome hit. Zero web calls.
```

## How It Connects

```
Web Search -> GENOME    (crystallized as permanent entry)
           -> INDEX     (source URL remembered)
           -> MEMORY    (exchange stored in .amem)
           -> CALIBRATION (tracks accuracy)

GitHub     -> GENOME    (approach extracted from code)
           -> SKILLS    (can generate new skill from repo)

Wikipedia  -> GENOME    (facts stored)
           -> BELIEFS   (updated from authoritative source)
```

The internet becomes part of Hydra. Not cached. Internalized. Permanent. Growing.
