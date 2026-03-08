# CLI Reference

## Installation

```bash
cargo install --path crates/hydra-cli
```

## Commands

### hydra run

Start a new task.

```bash
hydra run "Create a Python function to sort a list"
hydra run --profile performance "Analyze this large codebase"
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--profile` | `standard` | Resource profile (minimal, standard, performance, unlimited) |
| `--model` | auto | LLM provider (anthropic, openai, local) |
| `--budget` | 100000 | Token budget for this run |
| `--timeout` | 300 | Timeout in seconds |

### hydra status

Check the status of running tasks.

```bash
hydra status
hydra status <run-id>
```

### hydra cancel

Cancel a running task.

```bash
hydra cancel <run-id>
```

### hydra approve

Approve or deny a pending action.

```bash
hydra approve <approval-id>
hydra approve <approval-id> --deny
```

### hydra kill

Emergency stop.

```bash
hydra kill                    # Graceful stop
hydra kill --instant          # Immediate halt
hydra kill --freeze           # Freeze for inspection
```

### hydra health

Check system health.

```bash
hydra health
```

### hydra config

View or modify configuration.

```bash
hydra config show
hydra config set llm.provider anthropic
hydra config set limits.token_budget 200000
```

## Server Mode

Start Hydra as a persistent server:

```bash
cargo run -p hydra-server

# With custom port
HYDRA_PORT=8080 cargo run -p hydra-server

# With auth
AGENTIC_TOKEN=my-secret cargo run -p hydra-server
```

## Environment Variables

All configuration can be set via environment variables. See [Configuration](CONFIGURATION.md) for the full list.
