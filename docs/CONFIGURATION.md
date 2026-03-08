# Configuration

Hydra loads configuration from three sources (highest priority first):

1. **Environment variables** - Override everything
2. **Config file** - `~/.hydra/config.toml`
3. **Defaults** - Built-in sensible defaults

## Config File

Create `~/.hydra/config.toml`:

```toml
# Server
data_dir = "~/.hydra"
api_port = 7777
log_level = "info"          # trace, debug, info, warn, error
server_mode = false         # true = require auth token
profile = "standard"        # minimal, standard, performance, unlimited

# Voice
voice_enabled = false
wake_word = "hey hydra"

# LLM Providers
[llm]
anthropic_api_key = "sk-ant-..."
openai_api_key = "sk-..."
default_provider = "anthropic"
perception_model = "claude-haiku"    # Fast model for PERCEIVE phase
thinking_model = "claude-sonnet"     # Strong model for THINK phase
decision_model = "claude-haiku"      # Fast model for DECIDE phase

# Limits
[limits]
token_budget = 100000               # Max tokens per session
max_concurrent_runs = 10            # Parallel runs allowed
approval_timeout_secs = 300         # 5 min approval window
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `HYDRA_PORT` | `7777` | Server port |
| `HYDRA_DATA_DIR` | `~/.hydra` | Data directory |
| `HYDRA_LOG_LEVEL` | `info` | Log verbosity |
| `HYDRA_PROFILE` | `standard` | Resource profile |
| `HYDRA_VOICE` | `false` | Enable voice |
| `HYDRA_TOKEN_BUDGET` | `100000` | Token budget |
| `HYDRA_MAX_CONCURRENT_RUNS` | `10` | Max parallel runs |
| `HYDRA_SERVER_MODE` | `false` | Require auth |
| `AGENTIC_TOKEN` | (none) | Auth token (server_mode) |
| `ANTHROPIC_API_KEY` | (none) | Anthropic API key |
| `OPENAI_API_KEY` | (none) | OpenAI API key |

## Resource Profiles

| Profile | RAM | Use Case |
|---------|-----|----------|
| `minimal` | 256 MB | Low-resource devices, Raspberry Pi |
| `standard` | 512 MB | Default, most development |
| `performance` | 1 GB | Heavy workloads, many sisters |
| `unlimited` | No limit | Servers, CI/CD |

## API Key Setup

### Anthropic (recommended)

1. Go to [console.anthropic.com](https://console.anthropic.com)
2. Create an API key
3. Set it:
   ```bash
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

### OpenAI

1. Go to [platform.openai.com](https://platform.openai.com)
2. Create an API key
3. Set it:
   ```bash
   export OPENAI_API_KEY="sk-..."
   ```

### Multiple Providers

Both keys can be set simultaneously. The model router selects the best provider per task based on capability, cost, and latency scores.

## Model Selection

Hydra uses different models for different cognitive phases:

| Phase | Default | Purpose |
|-------|---------|---------|
| PERCEIVE | claude-haiku | Fast intent classification |
| THINK | claude-sonnet | Deep reasoning |
| DECIDE | claude-haiku | Quick action selection |
| ACT | (varies) | Depends on the action |
| LEARN | (none) | Sister calls only |

Override per phase in config:

```toml
[llm]
perception_model = "gpt-4o-mini"
thinking_model = "claude-sonnet"
decision_model = "gpt-4o-mini"
```

## Data Directory

Default: `~/.hydra/`

```
~/.hydra/
  config.toml       # Configuration
  hydra.db           # SQLite database
  checkpoint.json    # Last checkpoint (crash recovery)
  receipts/          # Action receipts
  evidence/          # Evidence attachments
  cache/             # Temporary cache
  logs/              # Log files
  voice/             # Voice recordings
```

## Server Mode

For remote access, enable server mode with auth:

```bash
export HYDRA_SERVER_MODE=true
export AGENTIC_TOKEN="your-secret-token"
hydra-server
```

Clients must include `Authorization: Bearer your-secret-token` in requests.
