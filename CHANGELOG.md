# Changelog

## March 22, 2026 — The Hands + Hardening Release

### New Crates (3)
- **hydra-browser** (2,668 lines) — Headless Chrome via CDP, page analysis, login automation with vault+TOTP, CAPTCHA solving (6 types), computer-use agent, VisionProvider trait
- **hydra-desktop** (1,382 lines) — Screen capture (macOS/Linux), input simulation (bezier mouse, typing cadence), app manager, window orchestrator, clipboard monitor
- **hydra-mcp** (1,354 lines) — MCP server (8 tools), MCP client (discover/call), JSON-RPC protocol, stdio + memory transport

### Universal Connector Infrastructure
- **BridgeManager** — spawn/stop/send/health-check/auto-restart bridge subprocesses
- **BridgeProcess** — subprocess lifecycle with reader thread, exponential backoff, graceful shutdown
- **BridgeConfig** — serde structs for bridge.toml parsing (runtime, transport, lifecycle, credentials)
- **LocalExecutor** — filesystem/http_local/applescript/subprocess execution with path traversal prevention
- **LocalConfig** — serde structs for local.toml parsing (access method, capabilities, watch)

### New Integrations (8 total new)
- **Telegram** — bridge.toml + bridge.js (node-telegram-bot-api)
- **Discord** — bridge.toml + bridge.js (discord.js)
- **WhatsApp** — bridge.toml + bridge.js (@whiskeysockets/baileys, QR pairing)
- **Slack** — bridge.toml + bridge.js (@slack/bolt, socket mode)
- **iMessage** — local.toml (AppleScript, macOS only)
- **Obsidian** — local.toml (filesystem access to vault)
- **Philips Hue** — local.toml (http_local to bridge)
- **File operations** — read/write/copy/move/delete/list/search/download

### Hardening (24 items completed)
- **Multi-line input** — Shift+Enter / Alt+Enter in TUI
- **Env var overrides** — HYDRA_THEME, HYDRA_LLM_PROVIDER, HYDRA_PACER_SPEED, etc.
- **Error display** — LLM errors humanized with actionable fixes
- **Skill validation** — situation/approach/confidence validated on load
- **Input sanitization** — control chars stripped, prompt injection logged
- **Audit hash chain** — Merkle-linked chain_hash + previous_hash on AuditRecord
- **Memory integrity** — SHA256 of .amem verified on boot
- **Vault encryption** — AES-256-GCM via HYDRA_VAULT_PASSPHRASE
- **Backup system** — /backup, /backup list, /backup restore, --backup CLI, auto-prune 30 days
- **Encrypted backup** — AES-256-GCM on backup archives
- **First-run wizard** — auto-detects first boot, prompts provider + API key
- **HTTP API** — axum server on port 3141 (/api/health, /api/status, /api/cycle)
- **API auth** — Bearer token from vault/hydra-api.toml
- **Remote client** — TUI --remote flag support module
- **Update command** — hydra --update checks GitHub releases
- **Conversation persistence** — exchanges saved to ~/.hydra/data/conversations/
- **Genome domain stats** — /genome domains shows per-domain counts
- **Memory age awareness** — older memories annotated in evidence
- **Behavioral self-test** — test questions with score tracking
- **Skill install from URL** — /skill install <url> downloads + validates
- **Prompt cache** — tracks system prompt hash, enables provider-side caching
- **Metrics dashboard** — /metrics shows genome, memory, DB sizes, tokens

### Infrastructure
- **Dockerfile** — multi-stage rust → debian:slim
- **docker-compose.yml** — volumes for data, skills, vault
- **scripts/install.sh** — universal installer (detect OS, build, create ~/.hydra)
- **.github/workflows/release.yml** — build 4 targets on tag push

### TUI Surface (29 commands)
All 29 slash commands wired end-to-end:
/help, /clear, /status, /self, /skills, /memory, /genome, /genome domains, /health, /web, /version, /theme, /settings, /profile, /dream, /compact, /copy, /export, /session, /skill, /skill install, /context, /stats, /voice, /pause, /resume, /digest, /inbox, /companion, /backup, /metrics, /quit

### Stats
- Workspace members: 74
- New code this session: ~12,000+ lines
- Tests: 21/21 E2E, 62/62 harness, 583+ unit
- Clippy: zero warnings
- All files under 400 lines
