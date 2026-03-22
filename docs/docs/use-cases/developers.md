---
title: "Developer Use Cases"
description: "Git mastery, system design, debugging, production ops -- 30 genome entries for senior engineering."
---

## 30 Proven Approaches for Developers

The `developer` skill contains 30 genome entries covering the skills that separate junior from senior engineers. Every entry is backed by tens of thousands of real-world observations.



  ### Git Mastery

    Interactive rebase, conflict resolution, undoing pushed commits, commit message discipline.
  
  ### System Design

    Scale calculations, real-time architecture, API design, design interview frameworks.
  
  ### Debugging

    Production bugs, memory leaks, performance profiling, structured logging strategies.
  
  ### Production Ops

    Incident response, deployment safety, monitoring, on-call runbooks.
  



## Git: What Seniors Know


  
**Clean History:**

    Interactive rebase onto target branch. Squash WIP commits into logical units. Each commit should compile and pass tests independently. Explain WHY in the message -- the diff shows WHAT.

    **Confidence: 0.92 | Observations: 100,000**
  
  
**Conflict Resolution:**

    `git merge --abort` first, understand both sides. Use `git log --merge` to see conflicting commits. Resolve file by file. Run tests before committing. Regenerate lock files rather than manual merge.

    **Confidence: 0.90 | Observations: 80,000**
  
  
**Undo a Push:**

    `git revert` for public branches (creates a new commit). Never `git reset --hard` on shared branches. If the commit contains secrets, the secret is already compromised -- rotate it.

    **Confidence: 0.94 | Observations: 50,000**
  


## System Design

```
Situation: "designing a system for millions of requests"
Approach:  Start with the math: RPS = daily users x actions / 86,400
           Identify the bottleneck (CPU, I/O, network, DB)
           Horizontal scaling for stateless, vertical for DB initially
           Cache everything read more than written
Confidence: 0.91 | Observations: 30,000
```

Key patterns in the genome:

- **Real-time systems**: WebSocket for bidirectional, SSE for server-to-client, polling for &lt;10s freshness
- **API design**: RESTful resources, versioning from day 1, cursor-based pagination, rate limiting with clear headers
- **Design interviews**: Requirements first (functional + non-functional), back-of-envelope math, high-level boxes, deep dive one component, explicit trade-offs

## Debugging at Senior Level


#### Production bugs that can't be reproduced locally

Add structured logging at entry, decision, and exit points. Deploy the logging. Wait for recurrence. Logs reveal the exact state. Reproduce locally with the exact input. Never use console.log -- use structured logging with correlation IDs.

**Confidence: 0.91 | Observations: 40,000**



#### Memory leaks in long-running services

Heap snapshot before and after the suspected operation. Compare: what grew? Common causes: event listener accumulation, cache without eviction, closures capturing outer scope. The leak is always in code that runs repeatedly, never in startup code.

**Confidence: 0.89 | Observations: 25,000**


## How It Works in Practice

```
You: "My pods keep crashing with OOMKilled"

Hydra:
  1. IDF scoring: "crashing" + "pods" + "OOMKilled"
     -> matches genome entry (conf=91%, obs=3,200)
  2. Proven approach surfaces in LLM prompt
  3. Response includes: "check resource requests vs limits --
     requests should be 80% of limits, not 50%"
  4. Answer is better than any LLM alone because it contains
     real operational experience
```


:::tip

The developer skill fuses with any other domain. Ask about "deploying a trading system" and Hydra combines developer expertise with finance knowledge automatically.

:::


## Coverage

The 30 entries span: git workflow, merge conflicts, commit hygiene, system design at scale, real-time architecture, API design, design interviews, production debugging, memory leaks, performance profiling, code review practices, incident response, deployment safety, monitoring strategy, and technical leadership.
