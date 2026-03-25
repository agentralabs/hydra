# Owner Guardrail System

**Status:** Complete
**Built:** 2026-03-25

## What It Does
Provides the owner with visibility and control over Hydra's self-governance without boxing its capabilities. Guardrails are additive (owner adds restrictions). Default = fully permissive. Hydra is limitless in what it CAN do. Guardrails govern what it does TO ITSELF.

## 4 Layers

### Layer 1: Kill Switch
- `~/.hydra/KILL` file — checked every ambient tick (~10s). Exists = graceful shutdown.
- Dead-man-switch — no owner interaction in N days → auto-pause (configurable).
- Remote HTTP kill — `POST /api/kill` with PIN authentication.

### Layer 2: Evolution Gates
- Approval queue — META1 proposes, owner reviews via `/evolution`.
- Forbidden paths — `security/`, `vault_crypto.rs`, `guardrail/` are immutable.
- Blast radius threshold — configurable in `boundaries.toml`.

### Layer 3: Audit Trail
- Append-only `~/.hydra/guardrails/audit.jsonl` — every decision logged.
- Hydra can read but never delete past entries.

### Layer 4: TUI Commands
- `/guardrail` — status, pause, resume, kill, reload
- `/evolution` — list, approve, reject proposals

## Crates Used
- hydra-kernel/src/guardrail/ (4 files, ~566 lines)
  - mod.rs — GuardrailEngine, state machine, kill/pause/resume
  - config.rs — GuardrailConfig, boundaries.toml parsing
  - audit.rs — AuditLog, append-only JSONL
  - evolution_gate.rs — EvolutionProposal, approval queue
- hydra-tui/src/v2/commands/guardrail.rs (180 lines)

## Dependencies
- Depends on: hydra-genome (evolution path checking), hydra-wisdom (blast radius)
- Required by: evolution/mod.rs (approval gate), loop_ambient.rs (kill check), loop_dream.rs (pause check), proactive.rs (pause check)

## Wiring (Law 10)
- Called from: ambient loop (kill + dead-man), dream loop (pause), evolution pipeline (approval), proactive engine (pause)
- TUI visible: /guardrail and /evolution commands, audit history
- Genome feedback: evolution approvals/rejections feed genome confidence

## Owner Quick Reference

| Emergency | Action | From |
|---|---|---|
| STOP NOW | `touch ~/.hydra/KILL` | Any terminal |
| Pause | `/guardrail pause` | TUI |
| Resume | `/guardrail resume` | TUI |
| Remote kill | `curl -X POST localhost:3141/api/kill -d '{"pin":"...","reason":"..."}'` | Any device |
| Review evolution | `/evolution` | TUI |
| Approve | `/evolution approve <id>` | TUI |
| Reject | `/evolution reject <id> reason` | TUI |
| Status | `/guardrail` | TUI |
| Reload config | `/guardrail reload` | TUI |
| Allow boot after kill | `rm ~/.hydra/KILL` then restart | Terminal |

## Configuration — `~/.hydra/guardrails/boundaries.toml`
```toml
# dead_man_switch_days = 7       # uncomment to enable
forbidden_paths = ["guardrail/", "security/", "vault_crypto.rs"]
max_lines_per_evolution = 400
require_approval_above = "Visible"
remote_kill_enabled = true
# http_kill_pin = "123456"        # uncomment for remote kill
```
