# Changelog

## [1.0.0] - 2026-03-07

### Added

- **Cognitive Loop** — 5-phase cycle (Perceive, Think, Decide, Act, Learn) with real LLM integration
- **JSON-RPC 2.0 API** — Methods: `hydra.run`, `hydra.cancel`, `hydra.kill`, `hydra.approve`, `hydra.status`, `hydra.health`
- **SSE Event Stream** — Real-time progress events for all cognitive phases
- **LlmPhaseHandler** — Real LLM calls per phase with configurable model selection
- **ModelExecutor** — Multi-provider LLM abstraction (Anthropic, OpenAI, Ollama) with capability-based routing
- **Kill Switch** — 3-level emergency stop: instant halt, graceful stop, freeze/resume
- **Approval Flow** — Pause runs for human approval with configurable timeout
- **Execution Gate** — 6-layer security with risk assessment and action filtering
- **Token Budget** — Per-session token tracking and enforcement
- **Sister Bridges** — MCP bridge infrastructure for 14 sisters (Memory, Identity, Vision, Codebase, Forge, Evolve, Cognition, Reality, Veritas, Time, Aegis, Rail, ACP, CLI)
- **Circuit Breaker** — Fault tolerance for model and sister connections
- **SQLite Persistence** — Runs, steps, approvals, and receipts
- **Boot Sequence** — Phased initialization with checkpoint recovery and orphan detection
- **Graceful Shutdown** — Checkpoint save, task cancellation, resource cleanup
- **Configuration** — File-based (`~/.hydra/config.toml`) + environment variable overrides
- **Resource Profiles** — minimal (256MB), standard (512MB), performance (1GB), unlimited
- **Server Mode** — Optional auth token for remote access
- **Action Compilation** — Zero-token execution for learned patterns
- **Skill Fabric** — Extensible, sandboxed skill system with MCP and OpenClaw adapters
- **P2P Federation** — Multi-agent collaboration, delegation, skill sharing
- **Animus Prime** — AI-native language with semantic AST, script parser, and 6-target compiler (JS, Python, Rust, Go, SQL, Shell)
- **Pulse** — Real-time tiered response with prediction and proactive suggestions
- **Observability** — Structured logging, metrics, distributed tracing, configurable exports
- **15 Advanced Capabilities** — Resurrection, Dream State, Shadow Self, Token Minimizer, Future Echo, Mutation, Forking
- **Desktop App** — Dioxus native app with globe UI and cognitive phase visualization
- **Documentation** — Quick start, API reference, architecture, configuration, troubleshooting, CLI, voice, skills, federation, offline, capabilities, Animus
- **CI/CD** — GitHub Actions for test, lint, and cross-platform build
- **1,395 tests** — Unit, integration, E2E, stress, contract, property, and hardening tests across 30+ crates

### Known Limitations (V1)

- Sister bridges use static tool names (live MCP discovery planned for V1.1)
- ServerPhaseHandler still active; LlmPhaseHandler swap is V1.1
- Approval flow is DB-based only (full async SSE flow in V1.1)
- Voice interface disabled (infrastructure present, not wired)
- No TLS/HTTPS (use reverse proxy for production)
