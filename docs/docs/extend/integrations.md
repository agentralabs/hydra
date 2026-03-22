---
title: "Integrations"
description: "Connect Hydra to any API with an api.toml file -- REST, GraphQL, WebSocket, and more."
---

## Connect to Anything. No Code.

Create a folder in `integrations/` with an `api.toml`. Hydra can now talk to that system.

```
integrations/
  alpaca/
    api.toml            <- describes the API endpoints
    credentials.toml    <- your API keys (never committed to git)
```

## api.toml Format

```toml
[integration]
name        = "alpaca"
description = "Stock trading brokerage -- paper and live trading"
protocol    = "REST"
base_url    = "https://paper-api.alpaca.markets/v2"
auth_type   = "header"
auth_header = "APCA-API-KEY-ID"

# What Hydra can READ (no approval needed)
[[capabilities.read]]
name        = "portfolio"
endpoint    = "/account"
method      = "GET"
description = "Get current portfolio value and buying power"

# What Hydra can WRITE (always requires approval)
[[capabilities.write]]
name              = "buy"
endpoint          = "/orders"
method            = "POST"
description       = "Place a buy order"
requires_approval = true
body_template     = '", "qty": "", "side": "buy"}'
```

## Read vs Write Capabilities



  ### Read Capabilities

    No approval needed. Hydra checks automatically. Examples: portfolio value, build status, service health.
  
  ### Write Capabilities

    Always require approval (enforced by constitution). Hydra asks before acting. Examples: place order, send message, create issue.
  



## Field Reference

| Field | Purpose | Examples |
|-------|---------|---------|
| `protocol` | Communication method | REST, GraphQL, WebSocket |
| `base_url` | API root URL | `https://api.example.com/v2` |
| `auth_type` | Authentication method | header, bearer, basic, query_param |
| `auth_header` | Header name for key | `Authorization`, `X-API-Key` |
| `body_template` | Request body with `` placeholders | JSON template |

## Credential Management

Credentials live in `credentials.toml` alongside the `api.toml`:

```toml
[credentials]
api_key    = "PK1234567890ABCDEF"
api_secret = "sk_live_abcdef1234567890"
```


:::warning

Credentials never leave your machine, never get committed to git, and never appear in logs, receipts, or memory. They are read-only by Hydra. For centralized credential management, see the [Vault](/extend/vault).

:::


## How Hydra Uses an Integration

```
You: "What is my portfolio worth?"

1. Comprehension: detects "portfolio" -> financial domain
2. Integration lookup: finds "alpaca" has capability "portfolio"
3. Credentials: reads api_key from vault/credentials
4. API call: GET /account
5. Response: "Portfolio value: $47,230. Buying power: $12,100."
6. Receipt: action logged with timestamp and result
7. Memory: result stored in .amem for future reference
```

## Shipped Integrations


  
**GitHub:**

    Repos, issues, PRs, CI status, code search across all public repos.
  
  
**Slack:**

    Read channel history, send messages (with approval).
  
  
**Stripe:**

    Balance, recent charges, active subscriptions.
  
  
**Datadog:**

    Time-series metrics, monitor status.
  


Every integration call produces a receipt with timestamp, endpoint, status code, and duration.
