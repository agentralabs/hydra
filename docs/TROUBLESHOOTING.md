# Troubleshooting

## Common Issues

### Server won't start

**"Address already in use"**

Another process is using port 7777.

```bash
# Find what's using the port
lsof -i :7777

# Use a different port
HYDRA_PORT=8888 hydra-server
```

**"No API key configured"**

Set at least one API key:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# or
export OPENAI_API_KEY="sk-..."
```

Or add to `~/.hydra/config.toml`:

```toml
[llm]
anthropic_api_key = "sk-ant-..."
```

### Runs fail immediately

**"Token budget exceeded"**

Your session has used all allocated tokens. Reset by restarting the server or increase the budget:

```bash
HYDRA_TOKEN_BUDGET=500000 hydra-server
```

**"No model available"**

The configured model isn't accessible. Check:
1. API key is valid and has credits
2. Model name is correct in config
3. Network connectivity to the provider

### SSE connection drops

**No events received**

```bash
# Verify the event stream is working
curl -N http://localhost:7777/events
```

You should see a `system_ready` event immediately. If not:
- Check that the server is running
- Check firewall/proxy settings
- Try connecting from localhost first

**Events stop mid-run**

The SSE connection may have timed out. Reconnect:

```bash
curl -N http://localhost:7777/events
```

Runs continue server-side even if the SSE connection drops.

### Approval timeouts

**"Approval expired"**

Approvals default to 5 minutes. Increase the timeout:

```toml
[limits]
approval_timeout_secs = 600  # 10 minutes
```

To respond to an approval:

```bash
curl -s -X POST http://localhost:7777/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"hydra.approve","params":{"approval_id":"<id>","decision":"approved"}}'
```

### Database errors

**"database is locked"**

Another Hydra instance may be running. Check:

```bash
ps aux | grep hydra-server
```

Only one instance should run per data directory.

**"migration failed"**

Delete the database and let Hydra recreate it:

```bash
rm ~/.hydra/hydra.db
hydra-server
```

### Build errors

**"failed to resolve workspace dependencies"**

```bash
cargo clean && cargo build --workspace
```

**"sqlx offline mode"**

Hydra uses sqlx with offline mode. If you see query errors:

```bash
# Rebuild with the checked-in query data
cargo build --workspace
```

## Debug Mode

Enable verbose logging:

```bash
HYDRA_LOG_LEVEL=debug hydra-server
```

For maximum detail:

```bash
HYDRA_LOG_LEVEL=trace hydra-server
```

Log levels: `error` < `warn` < `info` < `debug` < `trace`

## Health Check

Verify the server is healthy:

```bash
curl -s http://localhost:7777/health | jq .
```

Expected:

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

## Kill Switch

If Hydra is unresponsive or stuck, use the kill switch:

```bash
# Graceful stop (complete current phase)
curl -s -X POST http://localhost:7777/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"hydra.kill","params":{"level":"graceful","reason":"manual stop"}}'

# Instant halt (immediate stop)
curl -s -X POST http://localhost:7777/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"hydra.kill","params":{"level":"instant"}}'
```

## Getting Help

If none of the above resolves your issue:

1. Check server logs (`~/.hydra/logs/`)
2. Run with `HYDRA_LOG_LEVEL=trace` and capture output
3. Include the Hydra version (`hydra-server --version`)
