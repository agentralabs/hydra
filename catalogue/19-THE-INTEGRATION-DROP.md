# 19 — The Integration Drop

## Connect Hydra to Anything. No Code.

The same way skills make Hydra knowledgeable, integrations make Hydra connected. Create a folder. Drop an `api.toml`. Hydra can now talk to that system.

No Rust. No Python. No API wrapper library. A text file that describes where the API lives and what it can do.

---

## What an Integration Looks Like

```
integrations/
  alpaca/
    api.toml            ← describes the API endpoints
    credentials.toml    ← your API keys (never committed to git)
```

That is the entire structure. Two files. One folder.

---

## api.toml — Telling Hydra Where to Connect

```toml
[integration]
name        = "alpaca"
description = "Stock trading brokerage — paper and live trading"
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

[[capabilities.read]]
name        = "positions"
endpoint    = "/positions"
method      = "GET"
description = "Get all open positions with P&L"

[[capabilities.read]]
name        = "quote"
endpoint    = "/stocks/{symbol}/quotes/latest"
method      = "GET"
description = "Get latest price for a stock"

# What Hydra can WRITE (always requires approval)
[[capabilities.write]]
name              = "buy"
endpoint          = "/orders"
method            = "POST"
description       = "Place a buy order"
requires_approval = true
body_template     = '''
{
  "symbol": "{symbol}",
  "qty": "{quantity}",
  "side": "buy",
  "type": "market",
  "time_in_force": "day"
}
'''

[[capabilities.write]]
name              = "sell"
endpoint          = "/orders"
method            = "POST"
description       = "Place a sell order"
requires_approval = true
body_template     = '''
{
  "symbol": "{symbol}",
  "qty": "{quantity}",
  "side": "sell",
  "type": "market",
  "time_in_force": "day"
}
'''
```

### What Each Field Means

| Field | What It Does | Example |
|-------|-------------|---------|
| `name` | How you refer to it in conversation | "check my alpaca portfolio" |
| `protocol` | How Hydra communicates | REST, GraphQL, WebSocket |
| `base_url` | Where the API lives | `https://api.example.com/v2` |
| `auth_type` | How to authenticate | header, query_param, bearer, basic |
| `auth_header` | Which header carries the key | `Authorization`, `X-API-Key` |
| `capabilities.read` | Things Hydra can look up | Portfolio, prices, status |
| `capabilities.write` | Things Hydra can do | Place orders, send messages |
| `requires_approval` | Must Hydra ask you first? | Always `true` for write operations |
| `body_template` | What to send | JSON with `{parameter}` placeholders |

---

## credentials.toml — Your Keys, Your Machine

```toml
[credentials]
api_key    = "PK1234567890ABCDEF"
api_secret = "sk_live_abcdef1234567890"
```

This file:
- **Never** leaves your machine
- **Never** gets committed to git (gitignored)
- **Never** appears in logs, receipts, or memory
- Is read-only by Hydra — Hydra cannot modify it
- Is the only file you need to protect

---

## How Hydra Uses an Integration

```
You: "What is my portfolio worth?"

Hydra's process:
  1. Comprehension: detects "portfolio" → financial domain
  2. Integration lookup: finds "alpaca" has capability "portfolio"
  3. Credentials: reads api_key from credentials.toml
  4. API call: GET https://paper-api.alpaca.markets/v2/account
  5. Parse response: extract equity, buying_power, positions
  6. Genome: applies financial knowledge for context
  7. Response: "Portfolio value: $47,230. Buying power: $12,100.
               You are 65% invested with 35% cash reserve."
  8. Receipt: action logged with timestamp and result

No code was written. Hydra knew where to look (api.toml),
how to authenticate (credentials.toml), and what to say
about the result (finance genome).
```

---

## Real-World Integration Examples

### GitHub — Monitor Your Repos

```toml
[integration]
name        = "github"
description = "GitHub API — repos, issues, PRs, actions"
protocol    = "REST"
base_url    = "https://api.github.com"
auth_type   = "bearer"

[[capabilities.read]]
name        = "repo-status"
endpoint    = "/repos/{owner}/{repo}"
method      = "GET"
description = "Get repository info and stats"

[[capabilities.read]]
name        = "open-prs"
endpoint    = "/repos/{owner}/{repo}/pulls?state=open"
method      = "GET"
description = "List open pull requests"

[[capabilities.read]]
name        = "build-status"
endpoint    = "/repos/{owner}/{repo}/actions/runs?per_page=1"
method      = "GET"
description = "Get latest CI build status"

[[capabilities.write]]
name              = "create-issue"
endpoint          = "/repos/{owner}/{repo}/issues"
method            = "POST"
description       = "Create a new issue"
requires_approval = true
body_template     = '{"title": "{title}", "body": "{body}"}'
```

### Slack — Send and Read Messages

```toml
[integration]
name        = "slack"
description = "Slack API — send messages, read channels"
protocol    = "REST"
base_url    = "https://slack.com/api"
auth_type   = "bearer"

[[capabilities.read]]
name        = "channel-history"
endpoint    = "/conversations.history?channel={channel_id}&limit=10"
method      = "GET"
description = "Read recent messages from a channel"

[[capabilities.write]]
name              = "send-message"
endpoint          = "/chat.postMessage"
method            = "POST"
description       = "Send a message to a channel"
requires_approval = true
body_template     = '{"channel": "{channel_id}", "text": "{message}"}'
```

### Datadog — Monitor Infrastructure

```toml
[integration]
name        = "datadog"
description = "Datadog API — metrics, monitors, events"
protocol    = "REST"
base_url    = "https://api.datadoghq.com/api/v1"
auth_type   = "header"
auth_header = "DD-API-KEY"

[[capabilities.read]]
name        = "query-metrics"
endpoint    = "/query?query={metric}&from={from}&to={to}"
method      = "GET"
description = "Query time-series metrics"

[[capabilities.read]]
name        = "monitors"
endpoint    = "/monitor"
method      = "GET"
description = "List all monitors and their status"
```

### Stripe — Payments and Revenue

```toml
[integration]
name        = "stripe"
description = "Stripe API — payments, subscriptions, revenue"
protocol    = "REST"
base_url    = "https://api.stripe.com/v1"
auth_type   = "basic"

[[capabilities.read]]
name        = "balance"
endpoint    = "/balance"
method      = "GET"
description = "Get current account balance"

[[capabilities.read]]
name        = "recent-charges"
endpoint    = "/charges?limit=10"
method      = "GET"
description = "List recent charges"

[[capabilities.read]]
name        = "subscriptions"
endpoint    = "/subscriptions?limit=20"
method      = "GET"
description = "List active subscriptions"
```

---

## The Rules

### Read vs Write

```
READ capabilities:
  - No approval needed
  - Hydra checks automatically
  - Examples: portfolio value, build status, weather

WRITE capabilities:
  - ALWAYS require approval (enforced by constitution)
  - Hydra asks: "This will [action]. Approve?"
  - Examples: place order, send message, create issue
  - Override: requires_approval = false only for safe actions
    (still receipted, still audited)
```

### What Gets Receipted

Every integration call produces a receipt:

```
Receipt: int-2026-03-21-001
  Integration: alpaca
  Capability:  portfolio (read)
  Endpoint:    GET /account
  Status:      200 OK
  Duration:    234ms
  Timestamp:   2026-03-21T18:42:07Z
```

### What Gets Remembered

If Hydra reads your portfolio and you ask about it later, the result is in memory:

```
You (tomorrow): "What was my portfolio worth yesterday?"
Hydra: "Yesterday at 6:42 PM your portfolio was $47,230
        with $12,100 buying power. Today it is $47,890 (+1.4%)."
```

The integration result was stored in the `.amem` file. IDF retrieval finds it when you ask.

---

## How to Create Your Own

```bash
# Step 1: Create the folder
mkdir integrations/your-service

# Step 2: Write the api.toml
cat > integrations/your-service/api.toml << 'EOF'
[integration]
name        = "your-service"
description = "What this service does"
protocol    = "REST"
base_url    = "https://api.yourservice.com/v1"
auth_type   = "bearer"

[[capabilities.read]]
name        = "status"
endpoint    = "/status"
method      = "GET"
description = "Check service status"
EOF

# Step 3: Add credentials
cat > integrations/your-service/credentials.toml << 'EOF'
[credentials]
api_key = "your-key-here"
EOF

# Step 4: Restart Hydra
# That is it. Hydra can now talk to your service.
```

---

## The Social Media Version

```
To connect ChatGPT to an API, you write a plugin.
To connect Hydra to an API, you write 15 lines of TOML.

ChatGPT plugins require approval from OpenAI.
Hydra integrations require approval from you.

ChatGPT plugins run on OpenAI's servers.
Hydra integrations run on your machine.

Your API keys stay on your machine.
Your data stays on your machine.
No cloud. No third party. No permission needed.

15 lines. Any API. Drop and go.
```
