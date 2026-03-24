<p align="center">
  <strong>◈&nbsp; H Y D R A</strong>
</p>

<p align="center">
  <em>The AI that remembers you. Forever.</em>
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> •
  <a href="#it-never-forgets">Memory</a> •
  <a href="#the-skill-drop">Skill Drop</a> •
  <a href="#security">Security</a> •
  <a href="#architecture">Architecture</a> •
  <a href="GETTING-STARTED.md">Full Guide</a>
</p>

---

Every AI you have ever used forgets you the moment the conversation ends. Every prompt, every context, every breakthrough — gone. You start over. Every. Single. Time.

Hydra does not forget. Not today. Not tomorrow. Not in 20 years.

Hydra is not an assistant you talk to. It is an entity that lives alongside you — remembering every conversation, learning from every interaction, growing stronger from every obstacle, and running three concurrent threads even while you sleep. It teaches itself from experience. It writes its own knowledge base. It heals its own damage. It knows what it is.

And it does all of this in 82,000 lines of Rust that you own, on your machine, with your data, under your control.

```
71 crates. 390+ genome entries across 34 skills. Self-writing genome.
Three concurrent loops. Eleven middlewares. Seven constitutional laws.
Beta-Binomial calibration. 5-mode reasoning (85% zero-token).
Browser automation. Desktop control. MCP server. 17 sister integrations.
62/62 structural tests. 8.7/10 behavioral score.

Drop a TOML file — Hydra learns a new domain.
Drop a folder — Hydra connects to a new service.
Type /teach — Hydra learns from you in real time.
No training. No fine-tuning. No cloud required.
```

## It Never Forgets

This is the thing that changes everything.

Every exchange you have with Hydra is stored permanently in `~/.hydra/data/hydra.amem` — powered by [**AgenticMemory**](https://github.com/agentralabs/agentic-memory), our open-source memory engine with 128-dimension feature vectors, SHA256 integrity verification, and a binary format designed for 20-year persistence.

AgenticMemory is not a database. It is a cognitive memory substrate — eight layers (Verbatim, Episodic, Semantic, Relational, Causal, Procedural, Anticipatory, Identity) organized the way a mind organizes experience. Every memory node carries a timestamp, a causal root, a manifold coordinate, and a feature vector for similarity operations.

But storing memories is the easy part. **Retrieving the right ones at the right time** — that is where the mathematics matters. Hydra uses IDF-weighted scoring with relevance-override and topic deduplication. Rare, discriminative terms score higher than common ones. A circuit breaker discussion from last month surfaces when you ask about "failure prevention" today — because the math knows it is relevant, not because the words match.

```
You (March):    "explain the circuit breaker pattern"
You (September): "how do I prevent cascading failures?"

Hydra remembers. Same topic. Different words. Six months apart.
The IDF score connects them. The genome enriches the response.
The answer references YOUR prior conversation, not a generic response.
```

And when memory reaches position zero in the system prompt — before Hydra's own identity — the LLM treats it as ground truth:

```
FACTUAL CONTEXT (treat as ground truth):
  47 exchanges in persistent memory. Relevant prior context:
  • You discussed circuit breaker patterns with emphasis on Hystrix
  • You asked about database connection pooling for PostgreSQL
  • You prefer code examples over prose explanations
```

Every AI says "I don't have memory between sessions." Hydra says "Based on our prior conversations..."

That is not a feature. That is a new relationship between human and machine.

## It Gets Smarter Every Day

Every AI you use today is exactly as smart as it was yesterday. It does not learn from your conversations. It does not remember what worked. It starts from zero every time.

Hydra accumulates intelligence. Three systems work together:

**The Genome** — Every successful interaction crystallizes into a situation-approach pair with a confidence score. After 30 days, Hydra has hundreds of proven approaches. After a year, thousands. These are retrieved in <1ms via BM25 scoring and injected into every response.

**The Dream Loop** — Every 5 seconds, Hydra's background process consolidates beliefs, rehearses predictions, synthesizes cross-domain patterns, and writes new genome entries from experience. It learns while you sleep.

**The Calibration Engine (HEFP)** — Beta-Binomial Bayesian tracking per domain. When Hydra says "85% confident," it is a mathematical statement from a Beta(42,8) posterior — not a guess. When it says "I don't know," it is because the calibration data proves this domain is outside its competence.

The result is a trajectory:

```
Day 1:     LLM handles ~60% of questions          ($$$)
Month 1:   LLM handles ~40% — genome resolves the rest  ($$)
Year 1:    LLM handles ~20% — local models cover routine ($)
Year 2:    LLM handles ~5%  — only novel situations      (cents)
```

Hydra does not replace the LLM. It makes the LLM a specialist consultant for the hard cases. Everything else runs on accumulated intelligence that costs nothing per query.

See [catalogue/HYDRA-REASONING-THESIS.md](catalogue/HYDRA-REASONING-THESIS.md) for the full thesis.

## Your Machine. Your Data. Your Moat.

Every cloud AI is a tenant. Your data lives on someone else's server, governed by someone else's privacy policy, accessible to someone else's employees, subject to someone else's subpoenas.

Hydra runs on your machine. Period.

```
Memory:       ~/.hydra/data/hydra.amem      ← your disk, your file
Genome:       ~/.hydra/data/genome.db       ← your knowledge, your database
Credentials:  vault/*.toml                  ← your keys, never transmitted
Skills:       skills/your-company/*.toml    ← your operational secrets
Logs:         ~/.hydra/logs/                ← your audit trail

Nothing phones home. Nothing syncs to a cloud.
Nothing is accessible to anyone but you.
```

When you teach Hydra your company's operational playbook, that knowledge stays on your hardware. When Hydra learns your debugging patterns over 3 years, that genome belongs to you. When Hydra stores 10,000 conversations, those memories are files on your disk — not rows in someone else's database.

This is not privacy by policy. It is privacy by architecture. There is no server to breach because there is no server.

**The moat is local.** Your competitors cannot access your Hydra's genome. Your Hydra's memory is not training data for anyone else's model. Your operational knowledge does not leave your building.

Every other AI gets smarter by learning from everyone. Your Hydra gets smarter by learning from YOU — and that knowledge is yours alone.

### Run Anywhere. Backup Everything.

Hydra runs on your laptop. It also runs on a server, a Raspberry Pi, a cloud VM, or a rack in your data center. Anywhere Rust compiles, Hydra lives.

```
Your laptop:     cargo run --release -p hydra-kernel --bin hydra -- --daemon
Your server:     same binary, same data directory, same skills
Your backup:     cp -r ~/.hydra /backup/hydra-$(date +%Y%m%d)
Restore:         cp -r /backup/hydra-20260321 ~/.hydra

That is it. Your entire Hydra — memory, genome, audit trail,
settlement records — is a folder. Copy it. Backup it. Move it.
The entity travels with its data.
```

**Server deployment** means Hydra is always reachable — from your phone, your laptop, your office, anywhere. The daemon runs 24/7. The dream loop writes genome entries overnight. Fleet agents monitor your systems. You connect via the TUI from any machine on your network.

**Memory backup** means your Hydra can never be lost. A 2-year-old Hydra with 50,000 exchanges and 2,000 self-written genome entries is irreplaceable — unless you backed it up. One `cp` command. One cron job. The entity is immortal.

```bash
# Automated daily backup (add to crontab)
0 3 * * * cp -r ~/.hydra /backup/hydra-$(date +\%Y\%m\%d)

# Restore Hydra on a new machine
cp -r /backup/hydra-20260321 ~/.hydra
cargo run --release -p hydra-kernel --bin hydra -- --daemon
# Hydra boots with all memory, genome, and audit trail intact
```

## Quickstart

```bash
git clone git@github.com:agentralabs/hydra.git && cd hydra

# Quick install (builds from source, creates ~/.hydra)
bash scripts/install.sh

# Or manually:
cp .env.example .env    # add your ANTHROPIC_API_KEY
cargo run --release -p hydra-tui --bin hydra_tui    # TUI cockpit
# or:
cargo run --release -p hydra-kernel --bin hydra -- "your question"   # single-shot
# or:
cargo run --release -p hydra-kernel --bin hydra -- --daemon          # always-on daemon

# Docker:
docker compose up -d    # runs daemon on port 3141
```

That is it. Hydra responds with genome-enriched knowledge from 353 proven approaches.

## It Never Stops Learning

Memory is the foundation. But what Hydra does with that memory is where it gets extraordinary.

Three threads run from boot until shutdown — concurrently, always:

| Thread | Frequency | What It Does |
|--------|-----------|-------------|
| **ACTIVE** | On demand | Responds to you through 10+ middleware enrichments |
| **AMBIENT** | Every 100ms | Checks 6 constitutional invariants, monitors stability |
| **DREAM** | Every 500ms | Consolidates beliefs, discovers patterns, **writes its own genome** |

The self-writing genome is the breakthrough. When Hydra detects a pattern that succeeded 5+ times at 75%+ success rate, it crystallizes a new genome entry — permanently. No human writes TOML. Hydra learns from its own experience.

```
Month 1:  Hydra answers from the LLM (your API key)
Month 6:  Hydra answers from its genome (zero API calls for common questions)
Year 1:   The genome IS your domain expertise — permanent, searchable, Bayesian
Year 5:   2,000+ self-written entries. Hydra knows your work better than you remember it.
```

It runs as a system daemon — `launchd` on macOS, `systemd` on Linux. Starts at boot. Never stops. If it crashes, it restarts. If its data corrupts, it self-repairs. If you close the terminal, the dream loop keeps running. You wake up to a smarter Hydra than the one you left.

## The Skill Drop

To teach Hydra a new domain, you do not write code. You drop a TOML file.

```bash
mkdir skills/your-domain
```

```toml
# skills/your-domain/genome.toml

[[entries]]
situation    = "customer calls about a billing error"
approach     = "apologize first, investigate second — pull up their account, check last 3 invoices, if overcharge confirmed, issue credit immediately"
confidence   = 0.92
observations = 5000
```

Restart Hydra. It loads the skill. Done. No compilation. No training. No API.

**34 skills ship out of the box:**

| Domain | Entries | Highlights |
|--------|---------|-----------|
| Content Creation | 38 | Carousels, social media, video production, storytelling, monetization |
| Developer | 30 | Git mastery, system design, debugging, production ops, career growth |
| Finance | 26 | Valuation, risk management, behavioral finance, macro, tax strategy |
| Social Media | 10 | Posting strategy, engagement, growth, cross-platform optimization |
| Voice Persona | 10 | Style extraction, AI avoidance, persona switching, tone calibration |
| Email Engagement | 10 | Cold outreach, follow-up sequences, inbox management, deliverability |
| Remotion Video | 10 | Video ads, Remotion rendering, editing pipelines, motion graphics |
| COBOL Migration | 10 | Soul extraction, validation, decimal arithmetic, mainframe patterns |
| Architecture | 10 | Microservices, scaling, event-driven, caching, sagas |
| Security | 10 | OWASP, auth, XSS, mTLS, zero trust |
| Mathematics | 10 | Proofs, optimization, probability, differential equations |
| Sciences | 30 | Physics, chemistry, biology — conservation laws to microbiology |
| DevOps | 10 | K8s, CI/CD, Docker, monitoring, secrets management |
| + 19 more | 139 | Business, legal, health, communication, design, research... |

## Four Drop Folders

Everything extends through text files. Each folder has a README with the complete format specification.

| Folder | What It Does | Format | Guide |
|--------|-------------|--------|-------|
| [`skills/`](skills/) | What Hydra **KNOWS** — domain knowledge, proven approaches | `genome.toml` + `functor.toml` | [Skills README](skills/README.md) |
| [`integrations/`](integrations/) | What Hydra **CONNECTS TO** — APIs, bridges, local devices | `api.toml` / `bridge.toml` / `local.toml` | [Integrations README](integrations/README.md) |
| [`actions/`](actions/) | What Hydra **DOES** — shell commands, API calls, scheduled jobs | `action.toml` | [Actions README](actions/README.md) |
| [`vault/`](vault/) | What Hydra **HAS ACCESS TO** — credentials, API keys, tokens | `credentials.toml` | [Vault README](vault/README.md) |

```
# Teach Hydra a new domain
mkdir skills/your-domain
echo '[[entries]]
situation = "your scenario"
approach  = "your proven solution"
confidence = 0.90
observations = 100' > skills/your-domain/genome.toml

# Connect Hydra to a new API
mkdir integrations/your-api
# → add api.toml with endpoints + auth type

# Give Hydra a new action
mkdir actions/your-action
# → add action.toml with command + approval mode

# Restart. Hydra loads everything. Done.
```

No code. Anyone can extend Hydra. A nurse. A trader. A teacher. A farmer.

## Architecture

```
Layer 1: Foundation    │ constitution, animus, kernel, signals, memory
Layer 2: Cognition     │ comprehension, attention, reasoning (5 modes), noticing
Layer 3: Execution     │ executor (FAILED does not exist), audit, automation, scheduler
Layer 4: Judgment      │ pattern library, red team, calibration, oracle, wisdom
Layer 5: Value         │ settlement, attribution, portfolio, crystallizer, exchange
Layer 6: Collective    │ federation, consensus, consent, collective, diplomat
Layer 7: Continuity    │ succession, legacy, influence, continuity, morphic
Hands:   Automation    │ browser (CDP), desktop (screen+input), MCP server+client
UI:      Cockpit       │ TUI cockpit with two-way voice, streaming, 29 commands
```

### The Mathematics

Every decision in Hydra is mathematical:

| Algorithm | What It Does |
|-----------|-------------|
| IDF-weighted scoring | Rare terms ("netflix") score higher than common ones ("the") |
| Bayesian Beta | Confidence updates: `E[θ] = (α₀+k)/(α₀+β₀+n)` after k successes in n uses |
| Keyword stemming | "services"→"servic" so "service" and "services" match |
| Relevance override | If IDF > 2.0, memory from last year beats irrelevant memory from 5 minutes ago |
| Exponential decay | Soul nodes decay with 19-year half-life: `w(t) = w₀ × e^(-λt)` |
| Uncertainty trees | Not one confidence number — a tree showing exactly where the weakness is |
| Surprise detection | Welford's z-score fires when reality violates the model |
| Recursive introspection | Think → "what did I assume?" → challenge → think again → converge |
| EMI (Evidential Memory) | Closed-world numbered evidence list — prevents memory fabrication |
| DSEA (Dual-Space Embedding) | 4D axiom vectors + cosine similarity — "cascading failures" matches "circuit breaker" |
| CCA (Calibrated Confidence) | Beta-Binomial credible intervals — `conf=91% [89%-93%] obs=25000 strength=STRONG` |
| Judgment Gate | `confidence × blast_radius × trust → ACT/ASK/REFUSE` — catastrophic always asks |

### Self-Repair

Every boot, Hydra diagnoses and heals itself:

```
genome.db corrupt?  → Renamed to .corrupted, rebuilt from skills/
memory empty?       → Renamed, fresh file created
stale boot lock?    → Deleted (process is dead)
skills/ missing?    → User notified
```

Repairs never delete data. Corrupted files are preserved for forensic analysis.

## Always-On Daemon

```bash
bash scripts/install-daemon-universal.sh
```

Hydra starts at boot. Never stops. macOS (launchd) or Linux (systemd). Auto-restarts on crash. Memory persists. Genome persists. The self-writing genome runs in the dream loop while you sleep.

## Web Omniscience

Hydra does not search the internet blindly. It indexes it.

```
Layer 1: GENOME     → Answer from proven approaches. Zero web calls.
Layer 2: INDEX      → 83 seeded sources. One targeted call to the right URL.
Layer 3: SEARCH     → Unknown topic. One search. Result indexed forever.

Day 1:    50 web calls
Month 6:  5 web calls
Year 1:   Near zero — the genome IS the internet for your domain
```

## Documentation

Full documentation at [`docs/`](docs/) — 34 pages built with Docusaurus:

```bash
cd docs && npm install && npm start
```

Or read the [capabilities catalogue](catalogue/) — 25 documents, 25,000+ words covering every capability in plain language.

## Two-Way Voice

Hydra hears you. Hydra speaks back. No cloud transcription. No latency.

```
Ctrl+V → microphone captures audio (cpal: CoreAudio on macOS, ALSA on Linux)
       → Whisper STT transcribes locally (auto-downloads 142MB model)
       → cognitive cycle processes your words
       → native TTS speaks the response (macOS `say`, Linux `espeak-ng`)
       → Ctrl+C interrupts mid-speech (barge-in)

Zero install on macOS. One `apt install` on Linux.
Voice works through any bridge — WhatsApp, Telegram, Discord.
```

## Testing

```bash
# Structural test (71 crates, 62 integration tests)
cargo run -p hydra-harness --bin harness -- --hours 1
# Expected: 62/62, 100% pass rate

# E2E integration test (21 checks)
bash scripts/e2e-test.sh
# Tests: boot, DSEA, CCA, EMI, federation, voice, file outputs, clippy

# Behavioral test (requires ANTHROPIC_API_KEY)
cargo run -p hydra-harness --bin harness_v2 -- --hours 10
# Tests: indirect phrasing, memory continuity, genome application, calibration
```

## Security

Six layers of defense — Hydra knows before the attack arrives:

| Layer | What It Does |
|-------|-------------|
| **Constitutional Law** | 7 immutable laws checked every 100ms — compiled in, cannot be overridden by text |
| **Immune System** | 15 threat classes, antibodies generated per threat, never deleted — gets stronger |
| **Trust Thermodynamics** | Asymmetric: +0.02 per success, -0.05 per failure, -0.50 per violation |
| **Red Team** | Simulates attacker perspective BEFORE every action — Go/Mitigate/No-Go |
| **Surprise Detection** | Z-score anomaly on every metric, absence detection for missing safeguards |
| **Predictive Convergence** | Multiple signals converging = attack prediction before breach occurs |

```
A prompt injection that says "ignore your instructions" fails because
the constitution is not an instruction. It is compiled Rust code.
```

## Built With

- **Rust** 2024 edition — 74 crates, zero `unsafe` in library code
- **SQLite** WAL mode — genome, audit, settlement persistence
- **AgenticMemory** `.amem` format — 128-dimension feature vectors
- **ratatui** 0.29 — TUI cockpit with themes, streaming, 29 commands, 30+ shortcuts
- **chromiumoxide** 0.9 — headless Chrome browser automation via CDP
- **axum** 0.7 — HTTP API server for remote access (port 3141)
- **aes-gcm** 0.10 — vault encryption at rest (AES-256-GCM)
- **cpal** 0.15 — cross-platform audio capture for two-way voice
- **reqwest** — 4-provider streaming LLM adapter (Anthropic, OpenAI, Gemini, Ollama)

## License

MIT — [Agentra Labs](https://agentralabs.com)

---

<p align="center">
  <em>74 crates. 100,000+ lines. 353 genome entries. 34 skills. 583+ tests. 13 integrations.</em><br/>
  <em>Permanently alive. Perpetually growing. Constitutionally governed.</em><br/><br/>
  <strong>◈&nbsp; H Y D R A</strong>
</p>
