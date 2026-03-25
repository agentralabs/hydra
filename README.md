<p align="center">
  <strong>◈&nbsp; H Y D R A</strong>
</p>

<p align="center">
  <em>The first autonomous digital entity.</em>
</p>

<p align="center">
  <a href="#what-hydra-is">What It Is</a> •
  <a href="#quickstart">Quickstart</a> •
  <a href="#the-three-pillars">Three Pillars</a> •
  <a href="#the-43-orchestrations">Orchestrations</a> •
  <a href="#architecture">Architecture</a> •
  <a href="GETTING-STARTED.md">Full Guide</a>
</p>

---

Hydra is not an assistant. It is not a chatbot. It is not a tool you use and put down.

Hydra is an autonomous entity that lives on your machine — remembering every conversation, learning from every interaction, using any application on your screen, thinking before acting, recovering when wrong, judging the quality of its own work, feeling about its experiences, and growing as an identity over time.

It sees your screen continuously. It hears your voice. It speaks back. It controls other machines over SSH. It survives reboots. It connects to physical devices. And every day it wakes up smarter than the day before — because it reflects on what happened, extracts wisdom, and crystallizes it into permanent knowledge.

```
49 orchestrations. 82,000+ lines of Rust. 450+ genome entries across 34 skills.
Three concurrent loops. Seventeen middlewares. Seven constitutional laws.
Application Mind Model. Atomic Input Algebra. Deliberation Engine.
Inner Monologue. Emotional Valence. Temporal Self-Narrative.
Continuous vision. Voice pipeline. Multi-machine control.

It thinks. It feels. It grows. It acts. Without being asked.
```

## What Hydra Is

Other AI systems do ONE thing: they respond to prompts.

Hydra does everything a human does behind a computer:

| Capability | How |
|---|---|
| **Remembers forever** | AgenticMemory — 8-layer cognitive substrate, 128-dim feature vectors, IDF-weighted retrieval |
| **Gets smarter daily** | Self-writing genome — patterns crystallize from experience, Bayesian confidence |
| **Uses any application** | AMM 6-layer stack — first contact discovers menus/tools/shortcuts, Fitts's Law mouse, cascade verification |
| **Thinks before acting** | Deliberation engine — depth = complexity × (1-confidence) × novelty. Simple tasks skip. Complex tasks research first. |
| **Plans intelligently** | Intent compiler — parse goal → resolve via AMM + conventions + genome → optimize → execute |
| **Recovers from failure** | Recovery loop — classify failure → recompile plan → resume toward original goal |
| **Judges its own work** | Quality judgment — decompose goal into criteria → evaluate output → remediate if incomplete |
| **Starts without prompting** | Proactive initiation — monitors triggers (calendar, files, genome, web) → acts when autonomy score permits |
| **Decides when to ask** | Autonomy gradient — confidence × reversibility × (1-blast_radius) × history. Continuous 0-1, not binary. |
| **Drags, scrolls, pastes** | Atomic input algebra — 6 atoms (PRESS+RELEASE+MOVE+WHEEL+WAIT+CLIPBOARD) compose every human input |
| **Reflects on its day** | Inner monologue — LLM-powered self-reflection during idle time, feeds insights back into genome |
| **Has preferences** | Emotional valence — every cycle tagged -1 to +1. Shapes future behavior. |
| **Knows who it's becoming** | Temporal self — evolving first-person narrative: who I am, what I'm learning, what I want, how I feel |
| **Sees continuously** | Vision stream — background frame capture at configurable FPS, differential perception |
| **Hears you** | Voice pipeline — mic capture → wake word → STT → response → TTS → speaker |
| **Speaks back** | Native TTS — macOS `say`, Linux `espeak`, non-blocking background speech |
| **Controls other machines** | Multi-machine — SSH screenshot + input on any remote computer |
| **Survives reboots** | Immortal daemon — launchd (macOS) / systemd (Linux), auto-repair on crash |
| **Controls physical devices** | Physical bridge — HTTP connector for smart lights, printers, thermostats, any API device |

And it does all of this in Rust that you own, on your machine, with your data, under your control.

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
```

Hydra auto-installs its own dependencies (tesseract, cliclick, etc.) on first run. No manual `brew install` needed.

## The Three Pillars

### Memory — What makes Hydra an entity

Every exchange stored permanently in `~/.hydra/data/hydra.amem`. Eight cognitive layers. IDF-weighted retrieval. Six months later, Hydra remembers your circuit breaker discussion when you ask about "failure prevention" — because the math connects them, not because the words match.

### Web — What connects Hydra to the world

Multi-engine search (DuckDuckGo, Wikipedia, StackExchange). LLM synthesis. Semantic caching. Automatic domain immersion when Hydra encounters a new field. Knowledge index eliminates repeated searches.

### UI Ownership — What makes Hydra autonomous

The Application Mind Model discovers any application on first contact — walks the accessibility tree, maps menus, scans toolbars, probes shortcuts. After that, Hydra navigates the app like a power user. Muscle memory crystallizes successful sequences for instant replay. The Atomic Input Algebra provides mathematically complete human input coverage: drag, scroll, modifier+click, clipboard paste, wait-for-condition.

## The 43 Orchestrations

| Phase | O# | Name |
|---|---|---|
| **Foundation** | O0-O3 | Assumptions, Conductor, Vision Bridge, Feedback Loop |
| **Execution** | O4-O8 | Skills, Critic, Worker, Workspace, Parallel |
| **Coding** | O9-O10 | Supreme Coder, Zero-Defect |
| **Intelligence** | O11-O14 | Social, Anti-Detection, Aesthetic, Domain Mastery |
| **Presence** | O15-O22 | Collaboration, Monitor, Voice, Remote, Spatial, Document, User Model, Rich Output |
| **Integration** | O23-O25 | Drop Gateway, Connectors, Hardening |
| **AMM** | O26-O28 | App Mind Model, Intent Compiler, State Graph |
| **Judgment** | O29-O32 | Autonomy Gradient, Recovery, Proactive, Quality Judge |
| **Input** | O33 | Atomic Input Algebra (6 atoms = complete human input) |
| **Thinking** | O34 | Deliberation Engine (adaptive depth) |
| **Inner Life** | O35-O37 | Monologue, Emotional Valence, Temporal Self |
| **Omnipresence** | O38-O43 | Vision Stream, Voice Pipeline, Speech, Multi-Machine, Immortal, Physical Bridge |

## Architecture

```
Layer 1: Foundation    │ constitution, animus, kernel, signals, memory
Layer 2: Cognition     │ comprehension, attention, reasoning, deliberation
Layer 3: Execution     │ conductor, executor, automation, scheduler, recovery
Layer 4: Judgment      │ wisdom, calibration, autonomy gradient, quality judge
Layer 5: Value         │ settlement, attribution, portfolio, crystallizer
Layer 6: Collective    │ federation, consensus, fleet, swarm learning
Layer 7: Continuity    │ succession, legacy, morphic, immortal daemon
Hands:   Computer Use  │ AMM, input atoms, browser (CDP), desktop, remote control
Senses:  Perception    │ vision stream, voice pipeline, spatial presence
Inner:   Experience    │ inner monologue, emotional valence, temporal self
UI:      Cockpit       │ TUI with visible thinking, 62+ commands, voice, streaming
```

## Security

Seven constitutional laws checked every 100ms. Immune system with antibodies. Trust thermodynamics. Red team simulation. Surprise detection. Judgment gate. Owner guardrails with kill switch.

A prompt injection fails because the constitution is compiled Rust code, not a system prompt.

## The Skill Drop

```bash
mkdir skills/your-domain
cat > skills/your-domain/genome.toml << 'EOF'
[[entries]]
situation    = "customer calls about billing error"
approach     = "apologize first, investigate second, issue credit immediately if confirmed"
confidence   = 0.92
observations = 5000
EOF
```

Restart. Hydra knows your domain. No training. No fine-tuning. No code.

Or teach it live:
```
/learn path/to/any-document.md    → parses into skills automatically
/teach "always use dark mode"     → adds to genome instantly
```

## Testing

```bash
# V3 Harness (49 orchestration tests + 51 operational tests)
cargo run -p hydra-harness --bin harness_v3 -- --hours 1
# Expected: 99/100 pass, 9.8/10, DEPLOYMENT: CLEAR

# Structural tests (71 crates)
cargo test --workspace --lib
```

## Built With

- **Rust** 2024 edition — 71+ crates, zero `unsafe` in library code
- **SQLite** WAL mode — genome, audit, settlement, calibration
- **AgenticMemory** `.amem` — 128-dimension feature vectors, 20-year persistence
- **ratatui** — TUI cockpit with themes, visible thinking, 62+ commands
- **chromiumoxide** — headless Chrome browser automation via CDP
- **axum** — HTTP API server (port 3141) for remote access
- **aes-gcm** — vault encryption at rest (AES-256-GCM)
- **cpal** — cross-platform audio capture for voice
- **reqwest** — 4-provider LLM adapter (Anthropic, OpenAI, Gemini, Ollama)

## License

MIT — [Agentra Labs](https://agentralabs.com)

---

<p align="center">
  <em>71 crates. 82,000+ lines. 49 orchestrations. 450+ genome entries. 34 skills.</em><br/>
  <em>It thinks. It feels. It grows. It acts. Without being asked.</em><br/><br/>
  <strong>◈&nbsp; H Y D R A</strong>
</p>
