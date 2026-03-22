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
| WhatsApp | Bridge | QR pairing via Baileys | Planned |
| Telegram | Bridge | Bot API | Planned |
| Discord | Bridge | WebSocket gateway | Planned |
| Slack | Bridge | Bolt framework | Planned |
| Signal | Bridge | signal-cli subprocess | Planned |
| iMessage | Bridge | AppleScript (macOS) | Planned |
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
| Obsidian | Local | Vault directory | Planned |
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
| Philips Hue | Local | Bridge HTTP API | Planned |
| Home Assistant | API | REST API | Planned |
| 8Sleep | API | REST API | Planned |

### Tools & Automation

| Service | Type | Status |
|---------|------|--------|
| Browser | Built-in | Planned (hydra-browser) |
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
| Screen Capture | Built-in | Planned (hydra-desktop) |
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
description = "Telegram bot for two-way messaging"

[bridge]
runtime = "node"
entry = "bridge.js"
transport = "stdio"
incoming_format = "json"
outgoing_format = "json"

[bridge.incoming]
sender_field = "from"
content_field = "text"

[bridge.outgoing]
recipient_field = "chat_id"
content_field = "text"

[bridge.lifecycle]
auto_start = true
restart_on_crash = true

[credentials]
vault_key = "telegram"
```

### Local Connector (`local.toml`)
```toml
[integration]
name = "obsidian"
type = "local"
description = "Read and write Obsidian vault"

[local]
access_method = "filesystem"
vault_path = "~/Documents/Obsidian"
file_pattern = "**/*.md"

[local.capabilities]
read = true
write = true
create = true
delete = false
```

## How It Works

When Hydra boots, the executor loads all integration TOMLs from `integrations/`. For each:

- **API**: Registers endpoints. Calls them when the cognitive loop needs external data.
- **Bridge**: Launches subprocess. Maintains persistent connection. Routes incoming messages to the companion signal stream. Sends responses back through the bridge.
- **Local**: Watches directories or connects to local APIs. Reads/writes on demand.

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
