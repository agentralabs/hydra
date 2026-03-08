# Architecture

Hydra is a cognitive orchestrator that coordinates AI sisters through a 5-phase loop.

## System Overview

```
                         ┌─────────────────────┐
                         │     Client (CLI)     │
                         └──────────┬──────────┘
                                    │
                           JSON-RPC │ SSE
                                    │
                         ┌──────────▼──────────┐
                         │    hydra-server      │
                         │  (Axum HTTP + SSE)   │
                         └──────────┬──────────┘
                                    │
                         ┌──────────▼──────────┐
                         │   hydra-runtime      │
                         │  Boot · Shutdown     │
                         │  KillSwitch · Tasks  │
                         │  Approval · Config   │
                         └──────────┬──────────┘
                                    │
                         ┌──────────▼──────────┐
                         │    hydra-kernel      │
                         │   CognitiveLoop      │
                         │   PhaseHandler       │
                         │   ExecutionGate      │
                         └──────────┬──────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
             ┌──────▼──────┐ ┌─────▼──────┐ ┌──────▼──────┐
             │ hydra-model  │ │hydra-sisters│ │  hydra-db   │
             │ ModelExecutor│ │ McpBridges  │ │  SQLite     │
             │ LLM Routing  │ │ 14 Sisters  │ │  Runs/Steps │
             └──────────────┘ └────────────┘ └─────────────┘
```

## Crate Responsibilities

### hydra-server

HTTP layer. Receives JSON-RPC requests, dispatches to runtime, streams SSE events.

- `POST /rpc` — JSON-RPC 2.0 endpoint
- `GET /events` — SSE event stream
- `GET /health` — Health check
- Auth token validation (server mode)

### hydra-runtime

Orchestration and lifecycle management.

- **Boot sequence** — Initialization phases, checkpoint recovery, orphan detection
- **Shutdown** — Graceful stop, checkpoint save, resource cleanup
- **KillSwitch** — 3-level emergency stop (instant, graceful, freeze)
- **ApprovalManager** — Pause runs for human approval with timeout
- **TaskRegistry** — Track and cancel running tasks via CancellationToken
- **Config** — Load from file + env vars, validate, provide to subsystems
- **LlmPhaseHandler** — Real LLM calls for each cognitive phase

### hydra-kernel

Core cognitive loop logic. Framework-agnostic.

- **CognitiveLoop** — Drives the 5-phase cycle
- **PhaseHandler trait** — `perceive`, `think`, `decide`, `assess_risk`, `act`, `learn`
- **ExecutionGate** — Risk assessment, action filtering
- **TokenBudget** — Track and limit token consumption

### hydra-model

LLM abstraction layer.

- **ModelExecutor** — Execute completion requests against any provider
- **ModelRegistry** — Register providers with capability/cost/latency scores
- **Model routing** — Select best provider per task
- Supports Anthropic and OpenAI

### hydra-sisters

Bridge layer to the 14 sister MCP servers.

- **McpSisterBridge** — Static bridge with known tool names
- **LiveMcpBridge** (planned) — Real JSON-RPC connection with capability discovery
- **CircuitBreaker** — Fault tolerance for sister connections
- Sisters: Memory, Identity, Vision, Codebase, Forge, Evolve, Cognition, Reality, Veritas, Time, Aegis, Rail, ACP, CLI

### hydra-db

Persistence layer using SQLite.

- Runs, steps, approvals, receipts
- Migrations via sqlx
- Connection pooling

## Cognitive Loop

```
User Input
    │
    ▼
┌─────────┐
│ PERCEIVE│──▶ Intent classification, entity extraction
└────┬────┘    Model: fast (Haiku)
     │
     ▼
┌─────────┐
│  THINK  │──▶ Reasoning, planning, sister queries
└────┬────┘    Model: strong (Sonnet)
     │
     ▼
┌─────────┐
│ DECIDE  │──▶ Action selection, risk assessment, gate check
└────┬────┘    Model: fast (Haiku)
     │
     ▼
┌─────────┐
│   ACT   │──▶ Execute via sisters or LLM generation
└────┬────┘    Model: varies
     │
     ▼
┌─────────┐
│  LEARN  │──▶ Memory storage, belief updates, receipts
└─────────┘    No LLM (sister calls only)
```

Each phase:
1. Receives output from the previous phase
2. Optionally queries sisters for context
3. Calls the LLM (except LEARN)
4. Emits SSE events with real content
5. Passes structured output to the next phase

## Kill Switch

Three escalation levels:

| Level | Behavior | Resume? |
|-------|----------|---------|
| `instant` | Abort all tasks immediately | No |
| `graceful` | Complete current phase, then stop | No |
| `freeze` | Pause all tasks in place | Yes |

The kill switch integrates with:
- TaskRegistry (cancellation tokens)
- ApprovalManager (cancel all pending)
- CognitiveLoop (phase-level checks)

## Approval Flow

```
DECIDE phase
    │
    ▼
ExecutionGate evaluates risk
    │
    ├── Low risk ──▶ Proceed to ACT
    │
    ├── Medium risk ──▶ Request approval
    │                      │
    │                   SSE: approval_required
    │                      │
    │                   Wait (timeout: 5min)
    │                      │
    │                   ├── Approved ──▶ ACT
    │                   ├── Denied ──▶ Error
    │                   └── Timeout ──▶ Error
    │
    └── High risk ──▶ Block
```

## Data Flow

All persistence flows through hydra-db (SQLite):

- **Runs** — Created on `hydra.run`, updated through phases
- **Steps** — One per cognitive phase per run
- **Approvals** — Created in DECIDE, resolved by `hydra.approve`
- **Receipts** — Generated in LEARN phase with full audit trail

## Token Budget

Token usage is tracked per-run and per-session:

- Each LLM call reports tokens used
- PhaseHandler accumulates per-phase totals
- TokenBudget enforces session-wide limits
- Configurable via `HYDRA_TOKEN_BUDGET` (default: 100,000)
