<p align="center">
  <strong>◈&nbsp; H Y D R A</strong>
</p>

<p align="center">
  <em>A living digital entity. Not an assistant. Not a chatbot. An entity.</em>
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> •
  <a href="#what-hydra-is">What Is This</a> •
  <a href="#the-skill-drop">Skill Drop</a> •
  <a href="#architecture">Architecture</a> •
  <a href="GETTING-STARTED.md">Full Guide</a>
</p>

---

Hydra is a Rust-based autonomous entity that remembers everything, teaches itself from experience, and never stops running. Drop a TOML file — Hydra learns a new domain. No training. No fine-tuning. No code.

```
68 crates. 82,000 lines of Rust. 278 genome entries across 28 skills.
Three concurrent loops. Seven constitutional laws. Self-writing genome.
Always-on daemon. IDF-scored memory retrieval. Bayesian confidence.

It runs while you sleep. It learns while you work. It remembers for 20 years.
```

## Quickstart

```bash
git clone git@github.com:agentralabs/hydra.git && cd hydra
cp .env.example .env    # add your ANTHROPIC_API_KEY
cargo run --release -p hydra-kernel --bin hydra -- "what is the circuit breaker pattern?"
```

That is it. Hydra responds with genome-enriched knowledge from 278 proven approaches.

## What Hydra Is

Three threads run from boot until shutdown:

| Thread | Frequency | What It Does |
|--------|-----------|-------------|
| **ACTIVE** | On demand | Responds to you through 8 middleware enrichments |
| **AMBIENT** | Every 100ms | Checks 6 invariants, integrates the stability equation |
| **DREAM** | Every 500ms | Consolidates beliefs, discovers patterns, **writes its own genome** |

The self-writing genome is the key: when Hydra detects a pattern that succeeded 5+ times at 75%+ success rate, it crystallizes a new genome entry automatically. No human writes TOML. Hydra learns from experience.

```
Month 1:  Hydra answers from the LLM (your API key)
Month 6:  Hydra answers from its genome (zero API calls for common questions)
Year 1:   The genome IS your domain expertise — permanent, searchable, Bayesian
```

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

**28 skills ship out of the box:**

| Domain | Entries | Highlights |
|--------|---------|-----------|
| Content Creation | 38 | Carousels, social media, video production, storytelling, monetization |
| Developer | 30 | Git mastery, system design, debugging, production ops, career growth |
| Finance | 26 | Valuation, risk management, behavioral finance, macro, tax strategy |
| Architecture | 10 | Microservices, scaling, event-driven, caching, sagas |
| Security | 10 | OWASP, auth, XSS, mTLS, zero trust |
| Mathematics | 10 | Proofs, optimization, probability, differential equations |
| Physics | 10 | Conservation laws, thermodynamics, quantum, relativity |
| Chemistry | 10 | Reactions, bonding, kinetics, electrochemistry, safety |
| Biology | 10 | Genetics, evolution, immunology, ecology, microbiology |
| DevOps | 10 | K8s, CI/CD, Docker, monitoring, secrets management |
| + 18 more | 104 | Business, legal, health, communication, design, research... |

## Four Drop Folders

Everything extends through text files:

```
skills/          → What Hydra KNOWS        (genome.toml + functor.toml)
integrations/    → What Hydra CONNECTS TO  (api.toml — GitHub, YouTube, any API)
actions/         → What Hydra DOES         (shell commands, API calls, schedules)
vault/           → What Hydra HAS ACCESS   (credentials with permission gates)
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
UI:      Cockpit       │ TUI with thinking verbs, voice stub, companion
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

## Testing

```bash
# Structural test (68 crates, 47 integration tests)
cargo run -p hydra-harness --bin harness -- --hours 1
# Expected: 47/47, 100% pass rate

# Behavioral test (requires ANTHROPIC_API_KEY)
cargo run -p hydra-harness --bin harness_v2 -- --hours 10
# Tests: indirect phrasing, memory continuity, genome application, calibration
```

## The Roadmap to 9.9/10

Seven mathematical structures take Hydra from 8.1 to 9.9:

1. **Living Manifold** — Riemannian genome geometry
2. **Causal Tensor** — Bayesian network memory
3. **Anticipatory Field** — answers form before questions complete
4. **Morphic Attractor** — provably stable identity
5. **Eigenbeliefs** — PCA on the belief manifold
6. **Synthesis Operator** — mathematical invention
7. **Conformal Confidence** — provably calibrated prediction intervals

See [`HYDRA-ASTRAL-MATHEMATICS.md`](HYDRA-ASTRAL-MATHEMATICS.md) for the full specification.

## Built With

- **Rust** 2024 edition — 68 crates, zero `unsafe` in library code
- **SQLite** WAL mode — genome, audit, settlement persistence
- **AgenticMemory** `.amem` format — 128-dimension feature vectors
- **ratatui** — TUI cockpit with crossterm backend
- **reqwest** — 4-provider LLM adapter (Anthropic, OpenAI, Gemini, Ollama)

## License

MIT — [Agentra Labs](https://agentralabs.com)

---

<p align="center">
  <em>68 crates. 82,000 lines. 278 genome entries. One entity.</em><br/>
  <em>Permanently alive. Perpetually growing. Constitutionally governed.</em><br/><br/>
  <strong>◈&nbsp; H Y D R A</strong>
</p>
