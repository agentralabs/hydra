# API Reference

Hydra exposes a JSON-RPC 2.0 API over HTTP and real-time events via SSE.

**Base URL:** `http://localhost:7777`

---

## JSON-RPC Methods

All methods are called via `POST /rpc` with `Content-Type: application/json`.

### hydra.run

Start a cognitive loop for a task.

**Params:**
```json
{
  "intent": "string (required) - What you want Hydra to do"
}
```

**Response:**
```json
{
  "run_id": "uuid",
  "status": "accepted"
}
```

The run executes asynchronously. Monitor progress via SSE events.

---

### hydra.cancel

Cancel a running task.

**Params:**
```json
{
  "run_id": "string (required)"
}
```

**Response:**
```json
{
  "success": true
}
```

---

### hydra.kill

Emergency kill switch. Cancels all active runs.

**Params:**
```json
{
  "level": "instant | graceful | freeze (default: graceful)",
  "reason": "string (optional)"
}
```

**Response:**
```json
{
  "success": true,
  "level": "graceful",
  "cancelled_runs": 3
}
```

| Level | Behavior |
|-------|----------|
| `instant` | Stop everything immediately, no cleanup |
| `graceful` | Complete current phase, then stop |
| `freeze` | Pause all runs, resumable |

---

### hydra.approve

Respond to an approval request.

**Params:**
```json
{
  "approval_id": "string (required)",
  "decision": "approved | denied (required)"
}
```

**Response:**
```json
{
  "success": true
}
```

---

### hydra.status

Get status of runs.

**Params (optional):**
```json
{
  "run_id": "string (optional - omit for all runs)"
}
```

**Response:**
```json
{
  "runs": [
    {
      "id": "uuid",
      "intent": "string",
      "status": "pending | running | completed | failed | cancelled",
      "created_at": "ISO 8601",
      "steps": [
        {
          "id": "uuid",
          "description": "Perceiving intent",
          "status": "pending | running | completed | failed | skipped"
        }
      ]
    }
  ]
}
```

---

### hydra.health

Health check.

**Response:**
```json
{
  "status": "ok",
  "uptime_seconds": 3600,
  "sisters": {
    "memory": "not_connected",
    "identity": "not_connected"
  }
}
```

---

## Error Codes

| Code | Meaning |
|------|---------|
| `-32700` | Parse error (invalid JSON) |
| `-32600` | Invalid request (missing jsonrpc/method) |
| `-32601` | Method not found |
| `-32602` | Invalid params |
| `-32603` | Internal error |

---

## SSE Events

Connect to `GET /events` for real-time streaming.

```bash
curl -N http://localhost:7777/events
```

### Event Types

#### system_ready

Emitted on connection.

```json
{"version": "0.1.0"}
```

#### run_started

A new run has been accepted.

```json
{
  "run_id": "uuid",
  "intent": "string",
  "estimated_steps": 5
}
```

#### step_started

A cognitive phase has begun.

```json
{
  "run_id": "uuid",
  "step_id": "uuid",
  "phase": "perceive | think | decide | act | learn",
  "description": "Perceiving intent"
}
```

#### step_progress

Phase progress update.

```json
{
  "run_id": "uuid",
  "step_id": "uuid",
  "progress": 0.5,
  "phase": "think"
}
```

#### step_completed

A cognitive phase has finished.

```json
{
  "run_id": "uuid",
  "step_id": "uuid",
  "result": "success",
  "phase": "perceive"
}
```

#### approval_required

User approval needed before proceeding.

```json
{
  "run_id": "uuid",
  "approval_id": "uuid",
  "action": "delete file",
  "risk_score": 0.75,
  "reason": "Destructive operation",
  "expires_at": "ISO 8601"
}
```

#### run_completed

Run finished successfully.

```json
{
  "run_id": "uuid",
  "status": "success",
  "tokens_used": 1250
}
```

#### run_error

Run failed.

```json
{
  "run_id": "uuid",
  "error": "Error description"
}
```

#### heartbeat

Sent every 30 seconds.

```json
{"status": "alive"}
```

#### system_shutdown

Server is shutting down.

```json
{"reason": "User requested shutdown"}
```

---

## REST Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/health` | Simple health check (JSON) |
| `POST` | `/rpc` | JSON-RPC 2.0 endpoint |
| `GET` | `/events` | SSE event stream |
