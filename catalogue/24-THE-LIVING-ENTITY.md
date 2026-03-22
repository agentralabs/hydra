# 24 — The Living Entity

## Everything That Makes Hydra Alive — Built in One Session

This document covers every capability that was built but not yet catalogued in the other documents. These are the pieces that turn Hydra from a system into a living entity.

---

## Self-Knowledge

Hydra examines its own internal state and describes what it finds. Not a system prompt. Not a persona. Facts computed from real data.

```
"I am Hydra. I have 200 proven approaches in my genome.
 My strongest domain is finance with 26 entries at 90% confidence.
 My weakest is debugging with 8 entries.
 I have 94 persistent memories across sessions.
 My stability is 1.00 (stable).
 8 subsystems enrich every response I give."
```

Every sentence is a measurement, not a label. The `SelfPortrait` struct reads from the genome store, memory bridge, metabolism monitor, and middleware chain. If the genome grows, the description changes. If stability drops, the description reflects it.

**File:** `crates/hydra-kernel/src/self_knowledge.rs`

---

## Self-Repair

Hydra diagnoses and heals itself on every boot. Before any phase runs, the repair loop checks:

| Check | What It Detects | How It Repairs |
|-------|----------------|----------------|
| genome.db | SQLite corruption | Renames to .corrupted, rebuilds from skills/ |
| audit.db | SQLite corruption | Renames to .corrupted, fresh on next boot |
| hydra.amem | Empty file (0 bytes) | Renames to .corrupted, fresh file created |
| hydra.lock | Stale lock (>30s old) | Deleted (process is dead) |
| skills/ | Directory missing | Notifies user (cannot auto-fix) |

Key principle: **repairs never delete data.** Corrupted files are renamed with `.corrupted` extension. The original is preserved.

```
Boot log:
  hydra: boot self-repair: 1 fixed, 0 unresolved
  hydra: SELF-REPAIR — cleared stale lock at ~/.hydra/hydra.lock
  hydra: boot complete in 47ms (7 phases)
```

**File:** `crates/hydra-kernel/src/self_repair.rs`

---

## Self-Writing Genome

Every 20 ambient steps, the dream loop checks if the automation engine has detected patterns worth recording. A pattern becomes a permanent genome entry when:
- Observed **5+ times**
- Success rate **≥ 75%**

```
Week 3: Automation detects: "deployment safety asked 5 times,
         circuit breaker recommended each time, user confirmed 4/5"
Dream loop: creates genome entry automatically
         situation: "deployment safety concern"
         approach: "circuit breaker at every dependency boundary"
         confidence: 0.80 (4/5 confirmed)

Week 4: User asks about deployment.
         Hydra answers from GENOME, not LLM.
         Zero tokens. From its own experience.
```

The genome grows without anyone writing TOML. Hydra teaches itself.

**File:** `crates/hydra-kernel/src/loop_dream.rs` (lines 115-153)

---

## Surprise Detection

Hydra notices when reality violates expectations. Three detection modes:

| Mode | What It Detects | Math |
|------|----------------|------|
| Numeric | Value outside 2σ from running mean | Welford's z-score |
| Categorical | Novel category after 5+ observations | Set membership |
| Absence | Something expected is MISSING | Magnitude 3.0 always |

```
"You're asking about deployment but you have no rollback mechanism.
 SURPRISE (magnitude 3.0): expected 'rollback mechanism'
 (expected because: production deployment requires rollback).
 Observed: ABSENT."
```

The surprise detector runs in the intelligence middleware on every exchange.

**File:** `crates/hydra-noticing/src/surprise.rs`

---

## Recursive Introspection

After reaching a conclusion, Hydra questions it:

```
Iteration 1: "Deploy is safe" (confidence 0.85)
Iteration 2: "What does this depend on?"
             → "config correct" (0.50) ← load-bearing!
             → Removing this evidence drops confidence significantly
Iteration 3: Confidence adjusted to 0.78
             Converged.

Report: "Examined in 3 iterations. Challenged 1 assumption:
         'config correct (50%)'. Confidence: 78% (delta: -7%)."
```

The introspection loop removes each piece of evidence and checks if the conclusion still holds. Evidence that changes the conclusion significantly is flagged as "load-bearing."

**File:** `crates/hydra-reasoning/src/introspection.rs`

---

## Uncertainty Trees

Instead of one confidence number, a tree showing exactly where uncertainty lives:

```
Deploy safely (70%)
├── Tests pass (95%)
│   ├── Unit tests (99%)
│   └── Integration tests (85%)
└── Config correct (70%) ← WEAKEST LINK
    └── Secrets rotated (70%)

"Confidence: 70% (limited by: 'secrets rotated' at 70%)"
```

The weakest link tells you exactly what to fix. One number paralyzes. A tree gives you one action.

**File:** `crates/hydra-wisdom/src/uncertainty.rs`

---

## Constitutional Receipting

Four write sites are constitutionally protected:

| Write Site | Law Enforced | What Happens |
|-----------|-------------|--------------|
| Memory writes | Law 3 (Memory Sovereignty) | Provenance checked before every write |
| Belief revisions | Law 3 (Memory Sovereignty) | Provenance source required |
| Audit records | Law 1 (Receipt Immutability) | Constitutional check before append |
| Identity deepening | Law 2 (Identity Integrity) | Check before hash chain extension |

If any law is violated, the operation is **blocked** — not logged and continued.

**Files:** `memory/src/bridge.rs`, `belief/src/revision.rs`, `audit/src/engine.rs`, `morphic/src/identity.rs`

---

## Genome as Identity

The top genome entries are not advice Hydra follows. They are knowledge Hydra HAS:

```
System prompt (data-derived):
  "You are Hydra. You KNOW the following from direct operational experience:
   - Install circuit breakers at dependency boundaries (94% proven, 5000 obs)
   - Interface-first: define contracts before implementing (90% proven, 4000 obs)
   - Measure before optimizing (91% proven, 4000 obs)"
```

The LLM does not comply with these. It embodies them. The difference between following a manual and having experience.

**File:** `crates/hydra-kernel/src/loop_/middlewares/intelligence.rs`

---

## Five Alive Features

| Feature | What It Does | Where It Lives |
|---------|-------------|----------------|
| **Alert on surprise** | Notifies you when something critical is detected | intelligence middleware + alert action |
| **Ask for help** | Surfaces recurring knowledge gaps as questions | intelligence middleware + omniscience |
| **Curiosity** | Dream loop wonders about pattern connections | dream loop (step % 50) |
| **Weight of meaning** | Long sessions get deeper attention | intelligence middleware + prompt |
| **Evolved voice** | Adapts communication style to user | prompt builder tier |

---

## Always-On Daemon

Hydra runs as a system service. Starts at boot. Never stops.

```
macOS:  launchd → com.agentra.hydra.plist
Linux:  systemd → hydra.service
Both:   bash scripts/install-daemon-universal.sh

Three threads run forever:
  Active:  responds when you speak
  Ambient: checks health 10x/second
  Dream:   consolidates, explores, self-writes genome

Crashes: auto-restart. Memory persists. Genome persists.
```

**Files:** `com.agentra.hydra.plist`, `scripts/install-daemon*.sh`, `src/bin/hydra.rs` (daemon mode)

---

## The Four Drop Folders

```
skills/          → What Hydra KNOWS        (200 entries, 25 domains)
integrations/    → What Hydra CONNECTS TO  (web search, GitHub, Wikipedia)
actions/         → What Hydra DOES         (shell commands, API calls, scheduled tasks)
vault/           → What Hydra HAS ACCESS   (credentials with read/write/delete/spend gates)
```

No code. Just TOML files. Drop and go. Anyone can extend Hydra.

---

## The Executor Runtime

The executor actually runs actions and makes API calls:

- `execute_shell()` — runs shell commands from action.toml
- `execute_api_sync()` — calls HTTP APIs from integration api.toml
- `read_credentials()` — reads API keys from vault/*.toml
- `check_vault_permission()` — enforces read/write/delete/spend gates
- `substitute_params()` — fills `{param}` placeholders

Every execution is receipted. Every API key stays on your machine.

**File:** `crates/hydra-executor/src/runtime.rs`

---

## 200 Genome Entries Across 25 Skills

```
Engineering:    general(13) architecture(10) devops(10) coding(8) debugging(8)
Security:       security(10)
Science:        mathematics(10) physics(10) chemistry(10) biology(10) science(5)
Business:       finance(26) business(7) management(5) legal(5)
Human Skills:   communication(5) productivity(5) learning(5) education(5)
Creative:       design(5) humanities(5)
Research:       research(5) data-science(5)
Health:         health(5)
Web:            web-knowledge(8)

All 25 skills have functor.toml mappings.
Every skill is loaded on boot.
The self-writing genome adds more from experience.
```

---

## Signal Weight Fix

Constitutional signals now get a minimum weight of 0.85 regardless of causal chain depth. The constitution is the highest authority — never weakened by shallow chains.

**File:** `crates/hydra-animus/src/semiring/weight.rs`

---

## What This Session Built

```
New modules:
  self_knowledge.rs      — Hydra knows what it is
  self_repair.rs         — Hydra heals itself
  web_knowledge.rs       — Hydra indexes the internet
  runtime.rs             — Hydra executes actions and API calls
  introspection.rs       — Hydra questions its own thinking
  surprise.rs            — Hydra notices violations
  uncertainty.rs         — Hydra maps its own ignorance

Enhanced modules:
  loop_dream.rs          — self-writing genome
  engine.rs              — self-portrait, enrichment merge
  intelligence.rs        — surprise, gaps, weight, genome-as-identity
  prompt.rs              — memory at position 0, evolved voice, questions
  memory.rs              — IDF retrieval, topic dedup, relevance override
  bridge.rs              — persistent .amem, constitutional receipting
  boot.rs                — self-repair before phases
  hydra.rs (binary)      — daemon mode with three concurrent loops
  signature.rs           — keyword stemming
  store.rs               — IDF-weighted scoring
  entry.rs               — Bayesian Beta confidence
  node.rs (soul)         — exponential decay
  weight.rs (animus)     — constitutional signal minimum

New files:
  25 genome.toml files   — 200 entries of operational knowledge
  25 functor.toml files  — domain concept → axiom primitive mappings
  4 integration api.toml — web search, GitHub, Wikipedia, weather
  3 action.toml files    — alert, notify, scheduled check
  vault/                 — credential management with permission gates
  launchd plist          — macOS daemon
  systemd service        — Linux daemon
  install scripts        — universal daemon installer

Catalogue:
  24 documents, 25,000+ words

Tests:
  92 kernel tests pass
  68/68 crates pass all tests
  47/47 V1 harness
  Zero clippy warnings
```
