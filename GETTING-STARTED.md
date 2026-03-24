# Getting Started with Hydra

## First-Time Setup

### Option 1: Install Script (recommended)
```bash
git clone git@github.com:agentralabs/hydra.git && cd hydra
bash scripts/install.sh
```

### Option 2: Docker
```bash
git clone git@github.com:agentralabs/hydra.git && cd hydra
cp .env.example .env   # add your API key
docker compose up -d
```

### Option 3: Manual Build
```bash
git clone git@github.com:agentralabs/hydra.git && cd hydra
cargo build --release -p hydra-kernel -p hydra-tui
```

## Choose Your LLM Provider

| Provider | Env Variable | Model |
|----------|-------------|-------|
| Anthropic (default) | `ANTHROPIC_API_KEY` | claude-sonnet-4-20250514 |
| OpenAI | `OPENAI_API_KEY` | gpt-4o |
| Google Gemini | `GOOGLE_API_KEY` | gemini-pro |
| Ollama (local) | None needed | llama3 (or any local model) |

Set in `.env` or as environment variable:
```bash
export ANTHROPIC_API_KEY=sk-ant-your-key-here
export HYDRA_LLM_PROVIDER=anthropic
```

Override per-session with `HYDRA_*` env vars:
```bash
HYDRA_LLM_PROVIDER=ollama HYDRA_THEME=light hydra-tui
```

## First Run

On first boot with no `~/.hydra` directory, Hydra runs a setup wizard:
```
Welcome to Hydra.

LLM provider [anthropic/openai/ollama/gemini] (default: anthropic):
API key: sk-ant-...

Setup complete!
```

## Three Ways to Run

### TUI Cockpit (interactive)
```bash
hydra-tui
```

### Single-Shot (one question)
```bash
hydra "what is the circuit breaker pattern?"
```

### Daemon (always-on, background)
```bash
hydra --daemon
```
Daemon starts HTTP API on port 3141 and runs all three loops.

## Key TUI Commands

| Command | What It Does |
|---------|-------------|
| `/help` | Full command list |
| `/genome domains` | Per-domain genome stats |
| `/metrics` | System metrics dashboard |
| `/backup` | Create backup |
| `/backup list` | List backups |
| `/skill install <url>` | Install skill from URL |
| `/settings` | View/change settings |
| `/voice setup` | Set up voice (downloads whisper) |

**Multi-line input:** `Shift+Enter` or `Alt+Enter`

## Add a Skill

```bash
mkdir skills/my-domain
cat > skills/my-domain/genome.toml << 'EOF'
[[entries]]
situation    = "customer asks about refund policy"
approach     = "empathize first, explain 30-day window, offer store credit past 30 days"
confidence   = 0.90
observations = 500
EOF
```

Restart Hydra. Skill loaded.

## Connect a Service

Three connector types — all just TOML files:

| Type | File | Example |
|------|------|---------|
| API | `api.toml` | GitHub, Weather, YouTube |
| Bridge | `bridge.toml` + `bridge.js` | Telegram, Discord, WhatsApp, Slack |
| Local | `local.toml` | Obsidian, Philips Hue, iMessage |

See [integrations/README.md](integrations/README.md) for formats.

## Backup & Restore

```bash
/backup                          # create backup (TUI)
/backup list                     # list backups
/backup restore 2026-03-22       # restore from date
hydra --backup                   # create backup (CLI)
```

Enable encrypted backups:
```bash
export HYDRA_VAULT_PASSPHRASE=your-secret-passphrase
/backup    # now encrypted with AES-256-GCM
```

## Voice

```bash
/voice setup    # downloads whisper model (~142MB)
/voice test     # speak test phrase
# Ctrl+V to speak, Ctrl+C to interrupt
```

## Update

```bash
hydra --update            # check for latest release
bash scripts/install.sh   # rebuild from source
```

## Data Location

```
~/.hydra/
├── data/hydra.amem       # 20-year memory
├── data/genome.db        # genome knowledge
├── data/audit.db         # execution trail
├── backups/              # automated backups
├── config.toml           # settings
└── .env                  # provider config
```

## For Developers

See [CLAUDE.md](CLAUDE.md) for the 8 Implementation Laws and development rules.

```bash
bash scripts/e2e-test.sh                               # 21 E2E tests
cargo run -p hydra-harness --bin harness -- --hours 1  # 62 structural tests
cargo clippy --workspace -- -D warnings                # zero warnings
```
