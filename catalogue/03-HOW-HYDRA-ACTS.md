# 03 — How Hydra Acts

## The Principle

**FAILED does not exist as a state.** Every obstacle is navigational. Hydra does not fail — it reroutes. 13 approach types are attempted before a task is declared HardDenied, and HardDenied requires explicit constitutional evidence.

## The Task Engine

When you ask Hydra to do something, a `ManagedTask` is created with these states:

```
Active → Blocked → Rerouting → Active (cycle)
  ↓                                ↓
Suspended                       Complete
  ↓
  → HardDenied (constitutional stop — requires evidence)
```

Each task has 13 approach types it can cycle through:
1. Direct execution
2. Decompose into subtasks
3. Alternative tooling
4. Reduced scope
5. Escalate to principal
6. Retry with backoff
7. Skip and continue
8. Cache and defer
9. Parallel execution
10. External delegation
11. Approximation
12. Manual fallback
13. Constitutional override request

After all 13 are tried, the cycle counter increments and starts over. A task can cycle indefinitely. The only exit is completion or constitutional HardDenied.

## Execution Engine (hydra-executor)

Every action is:
1. **Registered** before it runs (receipted)
2. **Executed** with approach tracking
3. **Recorded** with outcome (Succeeded, Blocked, HardDenied)
4. **Settled** with cost accounting

The execution engine manages a registry of actions, each with parameters, approach types, and receipt chains. Maximum 256 concurrent tasks.

## Automation (hydra-automation)

*"You have done this 4 times this week. I can automate this."*

The automation engine observes execution patterns:
- Records every execution with domain, intent, success, duration
- Detects repeating patterns (same action, similar parameters)
- Proposes crystallization — never auto-crystallizes
- On approval: generates a valid skill package and hot-loads it

This is how Hydra learns new skills from its own behavior.

## Scheduling (hydra-scheduler)

Hydra acts when constraints fire — not just when asked:
- **Recurring jobs** — "Check build status every hour"
- **One-shot futures** — "Deploy at 3am when traffic is lowest"
- **Constraint activations** — "When error rate exceeds 5%, alert"
- **Metric conditions** — "When Lyapunov drops below 0.3, checkpoint"

Everything is receipted before firing.

## The Execution Stack

```
hydra-transform  — converts any data format through a universal intermediate
hydra-protocol   — adapts to any protocol (REST, gRPC, WebSocket, MQTT, ...)
hydra-reach      — connects to any device (same entity, same memory, anywhere)
hydra-reach-ext  — connects to any external system (relentless path escalation)
hydra-environment — detects what exists, adapts execution strategy
hydra-skills     — hot-loads capabilities with constitutional gating
hydra-horizon    — tracks expanding awareness and ability to act (only grows)
```

## Audit Trail (hydra-audit)

Every action produces a narrative:

*"12 attempts, 3 reroutes, 2 escalations, 1 completion. Total duration: 4.2 seconds. Total cost: 234 tokens. Receipt chain verified."*

Receipts are cryptographic primitives — SHA256 hashed, immutable, queryable forever. The audit engine makes them human-readable.

## The Crates

| Crate | Lines | Role |
|-------|-------|------|
| `hydra-executor` | 1,429 | Universal action engine — FAILED does not exist |
| `hydra-audit` | 1,695 | Execution accountability narrative |
| `hydra-automation` | 1,342 | Behavior crystallization — proposes, never auto-acts |
| `hydra-scheduler` | 1,336 | Temporal execution — acts when constraints fire |
| `hydra-transform` | 912 | Any data, any format, meaning preserved |
| `hydra-protocol` | 1,143 | Any protocol — discovers, adapts, reaches |
| `hydra-reach` | 998 | Universal device connectivity |
| `hydra-reach-extended` | 1,303 | External system connectivity — relentless |
| `hydra-environment` | 1,046 | Detect capabilities, adapt execution |
| `hydra-skills` | 566 | Hot-loadable skill substrate |
| `hydra-horizon` | 507 | Perception + action horizon (only expands) |

## In Plain Terms

Imagine someone who never gives up but also never charges ahead blindly. When a door is locked, they try 13 different ways to get through — pick the lock, find another door, ask someone with a key, climb through a window, call the locksmith, come back later, send someone else. They only stop if the constitution says "you may not enter this room."

And every single attempt is recorded in a permanent ledger that anyone can audit later.

That is how Hydra acts.
