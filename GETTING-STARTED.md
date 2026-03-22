# Getting Started with Hydra

## 3 Minutes to First Response

```bash
# 1. Clone
git clone git@github.com:agentralabs/hydra.git
cd hydra

# 2. Set your LLM key
cp .env.example .env
# Edit .env and add your ANTHROPIC_API_KEY

# 3. Build and run
cargo build --release -p hydra-kernel --bin hydra
cargo run --release -p hydra-kernel --bin hydra -- "what is the circuit breaker pattern?"
```

That is it. Hydra responds with genome-enriched knowledge from 278 proven approaches.

---

## Interactive Mode

```bash
cargo run --release -p hydra-kernel --bin hydra -- --interactive
```

```
you > what is the best approach to error handling?
hydra > Based on my operational experience (conf=92%, 30000 observations):
        match the codebase's existing error handling pattern...

you > /status
cognitive-loop: genome=278 soul=[ready] audit=[47 records] middlewares=8

you > exit
```

---

## TUI Cockpit

```bash
cargo run --release -p hydra-tui --bin hydra_tui
```

Full terminal interface with welcome screen, conversation stream, thinking verb animation, and status bar. Ctrl+C to exit cleanly.

---

## Always-On Daemon

```bash
# Install and start (detects macOS or Linux automatically)
bash scripts/install-daemon-universal.sh

# Check status
bash scripts/install-daemon-universal.sh status

# Hydra is now always running.
# Open the TUI to connect to it anytime.
```

---

## Teach Hydra Your Domain

Drop a folder in `skills/` with a `genome.toml`:

```bash
mkdir skills/your-domain

cat > skills/your-domain/genome.toml << 'EOF'
[[entries]]
situation    = "describe when this approach applies"
approach     = "describe the proven approach in detail"
confidence   = 0.90
observations = 1000
EOF
```

Restart Hydra. It loads your skill automatically. No code. No compilation.

See `skills/SKILL-FORMAT.md` for the complete guide including design systems, operations manuals, and glossaries.

---

## Connect Hydra to Any API

Drop a folder in `integrations/` with an `api.toml`:

```bash
mkdir integrations/your-service

cat > integrations/your-service/api.toml << 'EOF'
[integration]
name     = "your-service"
base_url = "https://api.example.com/v1"
auth_type = "bearer"

[[capabilities.read]]
name     = "status"
endpoint = "/status"
method   = "GET"
EOF
```

Add credentials to `vault/your-service.toml`. Restart Hydra.

---

## What Hydra Has Out of the Box

```
278 genome entries across 28 skills:
  Engineering: architecture, devops, security, coding, debugging
  Science: mathematics, physics, chemistry, biology
  Business: finance (26 entries), business, management, legal
  Content: content-creation (38 entries), video (10 entries)
  Developer: git mastery, system design, production ops, career growth
  Human: communication, productivity, learning, health

83 indexed knowledge sources (Rust docs, AWS, Wikipedia, etc.)
5 integrations ready (web search, GitHub, Wikipedia, YouTube, weather)
8 actions (alerts, video, carousel, social posts, scheduled tasks)
```

---

## Key Files

| File | What It Is |
|------|-----------|
| `catalogue/README.md` | Complete capability documentation (25 documents) |
| `skills/SKILL-FORMAT.md` | How to create skills for any domain |
| `HYDRA-ASTRAL-MATHEMATICS.md` | The roadmap from 8.1 to 9.9 |
| `vault/EXAMPLE.toml` | How to store credentials securely |

---

## Architecture at a Glance

```
68 crates. 82,000+ lines of Rust. 7 layers.

Three concurrent loops:
  ACTIVE:  responds to your input
  AMBIENT: checks health 10x/second
  DREAM:   learns while idle, writes its own genome

Seven constitutional laws:
  Every action receipted. Every claim attributed.
  Memory sovereign. Identity unforgeable.

Self-repair on every boot.
Self-knowledge from introspection.
Self-writing genome from experience.

The entity that never stops learning.
```
