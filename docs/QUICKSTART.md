# Quick Start

Get Hydra running in 5 minutes.

## Prerequisites

- **Rust** 1.75+ (`rustup update stable`)
- **API key**: Anthropic or OpenAI (at least one)

## Install

```bash
# Clone
git clone https://github.com/agentralabs/agentic-hydra.git
cd agentic-hydra

# Build
cargo build --workspace

# Or install the server binary
cargo install --path crates/hydra-server
```

## Configure

Set your API key:

```bash
# Anthropic (recommended)
export ANTHROPIC_API_KEY="sk-ant-..."

# Or OpenAI
export OPENAI_API_KEY="sk-..."
```

Or use a config file (`~/.hydra/config.toml`):

```toml
[llm]
anthropic_api_key = "sk-ant-..."
```

See [Configuration](CONFIGURATION.md) for all options.

## Run

```bash
# Start the server
hydra-server
# Output: Hydra server listening on 0.0.0.0:7777
```

## Send Your First Task

In another terminal:

```bash
curl -s -X POST http://localhost:7777/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "hydra.run",
    "params": {
      "intent": "What is 2 + 2?"
    }
  }' | jq .
```

Expected response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "run_id": "abc123...",
    "status": "accepted"
  }
}
```

## Watch Events

Stream real-time progress:

```bash
curl -N http://localhost:7777/events
```

You'll see events for each cognitive phase:

```
event: step_started
data: {"run_id":"abc123","phase":"perceive"}

event: step_completed
data: {"run_id":"abc123","phase":"perceive","result":"success"}

event: step_started
data: {"run_id":"abc123","phase":"think"}
...
```

## Check Status

```bash
curl -s -X POST http://localhost:7777/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"hydra.status","params":{}}' | jq .
```

## Stop a Run

```bash
curl -s -X POST http://localhost:7777/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"hydra.cancel","params":{"run_id":"abc123"}}' | jq .
```

## Next Steps

- [Configuration](CONFIGURATION.md) - Customize models, limits, profiles
- [API Reference](API.md) - All JSON-RPC methods and SSE events
- [Architecture](ARCHITECTURE.md) - How the cognitive loop works
- [Troubleshooting](TROUBLESHOOTING.md) - Common issues
