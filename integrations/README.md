# HYDRA INTEGRATIONS

## Works With Everything

Hydra connects to 50+ services through three connector types. Drop a TOML file. Hydra connects. That is it.

## Three Connector Types

| Type | File | What It Does | Example |
|------|------|-------------|---------|
| **API** | `api.toml` | REST/GraphQL request-response | GitHub, Gmail, Spotify, Twitter |
| **Bridge** | `bridge.toml` | Persistent real-time connection | WhatsApp, Telegram, Discord, Slack |
| **Local** | `local.toml` | Local apps, files, devices | Obsidian, Hue lights, 1Password |

## Quick Start

```bash
# Connect to a REST API
mkdir integrations/your-service
cat > integrations/your-service/api.toml << 'EOF'
[integration]
name = "your-service"
type = "api"
base_url = "https://api.example.com"
auth_type = "bearer"
credential_key = "your-service"

[[capabilities.read]]
name = "get-data"
endpoint = "/data?q={query}"
EOF

# Add credentials
cat > vault/your-service.toml << 'EOF'
[credentials]
api_key = "your-key-here"
EOF

# Restart Hydra. Done.
```

## Available Integrations

### Chat Providers — Message Hydra from anywhere

| Service | Type | Method | Status |
|---------|------|--------|--------|
| WhatsApp | Bridge | QR pairing via Baileys | ✅ Built (bridge.toml + bridge.js) |
| Telegram | Bridge | Bot API | ✅ Built (bridge.toml + bridge.js) |
| Discord | Bridge | WebSocket gateway | ✅ Built (bridge.toml + bridge.js) |
| Slack | Bridge | Bolt framework | ✅ Built (bridge.toml + bridge.js) |
| Signal | Bridge | signal-cli subprocess | Planned |
| iMessage | Local | AppleScript (macOS) | ✅ Built (local.toml) |
| Microsoft Teams | Bridge | Bot Framework | Planned |
| Matrix | Bridge | matrix-sdk | Planned |
| Nostr | Bridge | NIP-04 DMs | Planned |
| WebChat | API | Built into Hydra HTTP API | Planned |

### AI Models — Use any model, your keys

| Service | Type | Status |
|---------|------|--------|
| Anthropic (Claude) | API | ✅ Built (default) |
| OpenAI (GPT) | API | ✅ Built |
| Google (Gemini) | API | ✅ Built |
| Ollama (local) | API | ✅ Built |
| xAI (Grok) | API | Planned (OpenAI-compatible) |
| Mistral | API | Planned (OpenAI-compatible) |
| DeepSeek | API | Planned (OpenAI-compatible) |
| OpenRouter | API | Planned (unified gateway) |
| Perplexity | API | Planned |
| Hugging Face | API | Planned |

### Productivity — Notes, tasks, code

| Service | Type | Method | Status |
|---------|------|--------|--------|
| GitHub | API | REST API | ✅ Built |
| Obsidian | Local | Vault directory | ✅ Built (local.toml) |
| Notion | API | REST API | Planned |
| Trello | API | REST API | Planned |
| Apple Notes | Local | AppleScript | Planned |
| Apple Reminders | Local | AppleScript | Planned |
| Things 3 | Local | URL scheme | Planned |
| Bear Notes | Local | x-callback-url | Planned |

### Music & Audio

| Service | Type | Status |
|---------|------|--------|
| Spotify | API | Planned |
| Sonos | Local | Planned |
| Shazam | API | Planned |

### Smart Home

| Service | Type | Method | Status |
|---------|------|--------|--------|
| Philips Hue | Local | Bridge HTTP API | ✅ Built (local.toml) |
| Home Assistant | API | REST API | Planned |
| 8Sleep | API | REST API | Planned |

### Tools & Automation

| Service | Type | Status |
|---------|------|--------|
| Browser | Built-in | ✅ Built (hydra-browser) |
| Gmail | API | Planned |
| Cron | Built-in | ✅ Built (hydra-scheduler) |
| Webhooks | API | Planned |
| 1Password | Local | Planned (op CLI) |
| Weather | API | ✅ Built |
| Voice | Built-in | ✅ Built (hydra-voice) |

### Media & Creative

| Service | Type | Status |
|---------|------|--------|
| Image Gen | API | Planned (DALL-E/SD) |
| GIF Search | API | Planned (Giphy) |
| Screen Capture | Built-in | ✅ Built (hydra-desktop) |
| Video (Remotion) | Built-in | Planned |

### Social

| Service | Type | Status |
|---------|------|--------|
| Twitter/X | API | Planned |
| Email | Bridge | Planned (IMAP/SMTP) |

## File Formats

### API Connector (`api.toml`)
```toml
[integration]
name = "github"
type = "api"
base_url = "https://api.github.com"
auth_type = "bearer"
credential_key = "github"

[[capabilities.read]]
name = "search-repos"
endpoint = "/search/repositories?q={query}"
description = "Search repositories"
```

### Bridge Connector (`bridge.toml`)
```toml
[integration]
name = "telegram"
type = "bridge"
description = "Telegram messaging via Bot API"

[bridge]
runtime = "node"                         # node | python | binary | script
entry = "bridge.js"                      # relative to this directory
transport = "stdio"                      # stdio | websocket | unix_socket
auto_start = true
restart_on_crash = true
health_check_interval_seconds = 30
max_restart_attempts = 5

[bridge.incoming]
message_field = "text"
sender_field = "from"
timestamp_field = "timestamp"

[bridge.outgoing]
message_field = "text"
recipient_field = "chat_id"

[bridge.lifecycle]
init_command = '{"type":"init"}'
shutdown_command = '{"type":"shutdown"}'
health_command = '{"type":"ping"}'
health_response = '{"type":"pong"}'

[credentials]
vault_service = "telegram"
env_vars = ["TELEGRAM_BOT_TOKEN"]        # injected as env vars into subprocess
```

### Local Connector (`local.toml`)
```toml
[integration]
name = "obsidian"
type = "local"
description = "Read and write Obsidian vault notes"

[local]
access_method = "filesystem"             # filesystem | subprocess | http_local | applescript

[local.filesystem]
root_path = "~/Documents/Obsidian"
file_pattern = "*.md"
recursive = true

[local.capabilities]
read = true
write = true
create = true
delete = false                           # safety: no note deletion

[local.watch]
enabled = true
debounce_ms = 500
events = ["create", "modify"]
```

## How It Works

When Hydra boots, the executor loads all integration TOMLs from `integrations/`. For each:

- **API**: Registers endpoints from `api.toml`. Calls them when the cognitive loop needs external data. Auth resolved from `vault/` at runtime.
- **Bridge**: Spawns subprocess from `bridge.toml`. Reads JSON lines from stdout (incoming messages). Writes JSON lines to stdin (outgoing replies). Routes incoming messages to companion signal stream. Auto-restarts on crash with exponential backoff.
- **Local**: Dispatches operations from `local.toml`. Filesystem access with path traversal prevention. Local HTTP to smart home devices. AppleScript for macOS apps. Capability gating (read/write/create/delete permissions per connector).

Credentials are resolved from `vault/` at runtime — never stored in integration definitions.

Every integration response flows through the Judgment Gate:
- Contained actions (read data) → act autonomously
- Visible actions (post, reply) → ask for approval
- Catastrophic actions (delete, mass-send) → always ask

## Adding a New Integration

1. Create a folder: `mkdir integrations/your-service`
2. Add the right TOML file (`api.toml`, `bridge.toml`, or `local.toml`)
3. Add credentials: `vault/your-service.toml`
4. Restart Hydra

No code. No compilation. No deployment. Drop a folder. Hydra connects.

## Full Spec

See `specs/HYDRA-UNIVERSAL-CONNECTOR-SPEC.md` for the complete architecture, bridge runtime engine, and implementation plan.
