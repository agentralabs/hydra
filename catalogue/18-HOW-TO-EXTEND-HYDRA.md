# 18 — How to Extend Hydra Without Touching Code

## The Principle

Hydra was built so you never need to open a `.rs` file to make it smarter or give it new capabilities. Everything goes through drop folders. Knowledge goes in `skills/`. Connections go in `integrations/`. Actions go in `actions/`. Hydra picks them up on boot. No compilation. No deployment. No code.

This document covers how to plug anything into Hydra — trading, video generation, social media, monitoring, anything — without touching the brain.

---

## THE THREE DROP FOLDERS

```
hydra/
  skills/          ← What Hydra KNOWS (genome.toml + functor.toml)
  integrations/    ← What Hydra can CONNECT TO (api.toml)
  actions/         ← What Hydra can DO (action.toml)
```

**Skills** = knowledge. "If you see X, the proven approach is Y."
**Integrations** = connections. "Here is how to talk to Alpaca / YouTube / Slack."
**Actions** = capabilities. "When triggered, execute this command/API call."

Skills exist today. Integrations and actions are the next step — same TOML-drop pattern, zero code.

---

## PART 1: INTEGRATIONS — How to Connect Hydra to Anything

### The Pattern

Create a folder in `integrations/` with an `api.toml`:

```
integrations/
  alpaca/
    api.toml          ← connection details
    credentials.toml  ← secrets (gitignored, local only)
  youtube/
    api.toml
    credentials.toml
  slack/
    api.toml
    credentials.toml
```

### api.toml Format

```toml
[integration]
name        = "alpaca"
description = "Stock trading brokerage API"
protocol    = "REST"
base_url    = "https://paper-api.alpaca.markets/v2"
auth_type   = "header"
auth_header = "APCA-API-KEY-ID"
docs_url    = "https://docs.alpaca.markets"

# What this integration can provide (read)
[[capabilities.read]]
name        = "portfolio"
endpoint    = "/account"
method      = "GET"
description = "Get current portfolio value and buying power"

[[capabilities.read]]
name        = "positions"
endpoint    = "/positions"
method      = "GET"
description = "Get all open positions"

[[capabilities.read]]
name        = "quote"
endpoint    = "/stocks/{symbol}/quotes/latest"
method      = "GET"
description = "Get latest price for a stock"

# What this integration can do (write)
[[capabilities.write]]
name        = "buy"
endpoint    = "/orders"
method      = "POST"
description = "Place a buy order"
requires_approval = true     # Hydra asks you before executing
body_template = '''
{
  "symbol": "{symbol}",
  "qty": "{quantity}",
  "side": "buy",
  "type": "market",
  "time_in_force": "day"
}
'''

[[capabilities.write]]
name        = "sell"
endpoint    = "/orders"
method      = "POST"
description = "Place a sell order"
requires_approval = true
body_template = '''
{
  "symbol": "{symbol}",
  "qty": "{quantity}",
  "side": "sell",
  "type": "market",
  "time_in_force": "day"
}
'''
```

### credentials.toml (gitignored, never committed)

```toml
[credentials]
api_key    = "your-alpaca-key"
api_secret = "your-alpaca-secret"
```

### How Hydra Uses It

```
You: "What is my portfolio worth?"
Hydra: reads integrations/alpaca/api.toml
       finds capability: portfolio (read, GET /account)
       reads credentials.toml for auth
       makes the API call
       returns: "Portfolio value: $47,230. Buying power: $12,100."

You: "Buy 10 shares of AAPL"
Hydra: reads capability: buy (write, requires_approval=true)
       Hydra: "This will place a market buy order for 10 AAPL. Approve?"
       You: "yes"
       Hydra: executes, receipts the action
       returns: "Order placed. 10 AAPL at market. Receipt: abc123."
```

---

## PART 2: ACTIONS — What Hydra Can Do

### The Pattern

Create a folder in `actions/` with an `action.toml`:

```
actions/
  generate-video/
    action.toml
  deploy/
    action.toml
  send-slack/
    action.toml
```

### action.toml Format

```toml
[action]
name        = "generate-video"
description = "Generate a Remotion video using Claude Code"
trigger     = "manual"       # manual | scheduled | conditional
approval    = "required"     # required | auto | notify

# The command to execute
[action.execute]
type    = "shell"
command = '''
cd /path/to/remotion-project && \
npx remotion render src/index.tsx \
  --props='{"title": "{title}", "content": "{content}"}' \
  out/{output_filename}.mp4
'''
timeout_seconds = 300

# Parameters that Hydra fills in from the conversation
[[action.parameters]]
name     = "title"
type     = "string"
required = true

[[action.parameters]]
name     = "content"
type     = "string"
required = true

[[action.parameters]]
name     = "output_filename"
type     = "string"
default  = "output"
```

### Another Example: Slack Notification

```toml
[action]
name        = "send-slack"
description = "Send a message to a Slack channel"
trigger     = "manual"
approval    = "auto"        # auto-approve for notifications

[action.execute]
type    = "api"
method  = "POST"
url     = "https://hooks.slack.com/services/{webhook_path}"
headers = { "Content-Type" = "application/json" }
body    = '{"text": "{message}", "channel": "{channel}"}'

[[action.parameters]]
name     = "message"
type     = "string"
required = true

[[action.parameters]]
name     = "channel"
type     = "string"
default  = "#general"
```

### Another Example: Scheduled Monitoring

```toml
[action]
name        = "check-build"
description = "Check CI build status on GitHub"
trigger     = "scheduled"
schedule    = "every 30 minutes"
approval    = "notify"      # run automatically, notify on failure

[action.execute]
type    = "api"
method  = "GET"
url     = "https://api.github.com/repos/{owner}/{repo}/actions/runs?per_page=1"
headers = { "Authorization" = "Bearer {github_token}" }

[action.on_result]
if_contains  = "failure"
then         = "alert"
alert_message = "Build failed for {owner}/{repo}"
```

---

## PART 3: PUTTING IT ALL TOGETHER

### Example: Hydra as a Trading Assistant

```
integrations/
  alpaca/
    api.toml          ← connects to brokerage
    credentials.toml  ← API keys

skills/
  finance/
    genome.toml       ← 26 entries of Wall Street wisdom
    functor.toml      ← maps financial terms to risk primitives

actions/
  place-trade/
    action.toml       ← executes buy/sell with approval
  set-stop-loss/
    action.toml       ← sets protective stop
  portfolio-report/
    action.toml       ← generates daily summary
```

```
You: "Should I buy NVDA?"
Hydra: [knowledge] Checks genome: valuation approaches, moat analysis
       [integration] Calls Alpaca: gets current price, your portfolio
       [judgment] Red-team: identifies semiconductor cycle risk
       [wisdom] "NVDA P/E is 65, well above historical average.
                 Your portfolio is already 15% tech. Adding more
                 increases concentration risk. If you proceed,
                 position size should be max 2% of capital with
                 a stop-loss at -8%."
You: "Go ahead, 2% position with 8% stop"
Hydra: [action] Calculates: 2% of $47,230 = $944.60
       [action] "This will buy ~$945 of NVDA with an 8% stop-loss. Approve?"
You: "yes"
Hydra: [execute] Places order via Alpaca API
       [execute] Sets stop-loss via Alpaca API
       [receipt] "Bought 7 shares NVDA at $134.95. Stop-loss set at $124.15.
                  Receipt: tx-2026-03-21-001. Total invested: $944.65."
```

### Example: Hydra Generating Videos

```
skills/
  video/
    genome.toml       ← video creation approaches

actions/
  generate-video/
    action.toml       ← runs Remotion render command
```

```
You: "Create a video about how Hydra's memory works"
Hydra: [knowledge] Pulls from catalogue/05-HOW-HYDRA-REMEMBERS.md
       [action] Fills parameters: title="Hydra Memory", content=summary
       [approval] "This will render a Remotion video. Approve?"
You: "yes"
Hydra: [execute] Runs npx remotion render
       [receipt] "Video generated: out/hydra-memory.mp4 (2:34, 1080p)"
```

### Example: Hydra Monitoring Everything

```
integrations/
  github/api.toml
  datadog/api.toml
  slack/api.toml

actions/
  check-build/action.toml       ← every 30 min
  check-uptime/action.toml      ← every 5 min
  alert-slack/action.toml       ← on failure
```

```
[No user interaction needed]

Fleet Agent A: checks build every 30 min via GitHub integration
Fleet Agent B: checks uptime every 5 min via Datadog integration
Fleet Agent C: watches for alerts from A and B

3:17 AM — Agent A detects: build failed
          Agent A triggers: alert-slack action
          Slack: "Build failed on main at 3:17 AM. Test: test_auth_refresh.
                  This matches the token rotation issue from Tuesday."

Morning — You open Hydra
          Briefing: "▲ URGENT: Build failed at 3:17 AM — auth-service"
```

---

## PART 4: THE RULES

### What Requires Approval

```
ALWAYS requires approval (requires_approval = true):
  - Spending money (trades, purchases)
  - Sending messages to others (Slack, email)
  - Modifying external systems (deploy, database changes)
  - Deleting anything

NEVER requires approval:
  - Reading data (portfolio value, build status, metrics)
  - Internal computation (analysis, reasoning, pattern matching)
  - Writing to Hydra's own memory

CONFIGURABLE:
  - Notifications (can be auto or require approval)
  - Scheduled tasks (can be auto with notify-on-failure)
```

### Constitutional Enforcement

Every integration call and action execution is:
1. **Checked** against the 7 constitutional laws before execution
2. **Receipted** with SHA256 hash after execution
3. **Audited** in the settlement ledger with cost attribution
4. **Logged** in the memory bridge for future reference

```
No action happens in the dark.
No action happens without a receipt.
No action happens that violates the constitution.
```

### Security

```
credentials.toml is:
  - gitignored (never committed)
  - local only (stays on your machine)
  - read-only by Hydra (cannot be modified by actions)
  - encrypted at rest (future: via hydra-vault)

API keys never appear in:
  - logs
  - receipts
  - memory
  - genome entries
  - any transmitted data
```

---

## PART 5: HOW TO ADD YOUR OWN

### Step 1: Decide what you want

```
"I want Hydra to post on Twitter"
  → Integration: Twitter API
  → Action: send-tweet
  → Skill: social-media genome (optional, for voice/tone)

"I want Hydra to monitor my servers"
  → Integration: your monitoring API (Datadog, Grafana, custom)
  → Action: check-health (scheduled)
  → Action: alert-on-failure (conditional)

"I want Hydra to manage my calendar"
  → Integration: Google Calendar API
  → Action: create-event
  → Action: daily-briefing (scheduled, 8 AM)
```

### Step 2: Create the files

```bash
# Integration
mkdir integrations/twitter
# Edit integrations/twitter/api.toml (endpoints, auth)
# Edit integrations/twitter/credentials.toml (API keys)

# Action
mkdir actions/send-tweet
# Edit actions/send-tweet/action.toml (command or API call)

# Optional: Knowledge
# Edit skills/social-media/genome.toml (posting approaches)
```

### Step 3: Restart Hydra

```bash
cargo run -p hydra-tui --bin hydra_tui
# or
cargo run -p hydra-kernel --bin hydra -- --interactive
```

Hydra loads everything from the drop folders on boot. No code changes.

### Step 4: Use it

```
You: "Post about our new release on Twitter"
Hydra: [knowledge] Checks social-media genome for tone
       [action] Composes tweet from conversation context
       [approval] "Post this tweet? [shows text]. Approve?"
You: "yes"
Hydra: [execute] Calls Twitter API via integration
       [receipt] "Tweet posted. Receipt: tw-2026-03-21-001."
```

---

## WHAT EXISTS TODAY VS WHAT NEEDS TO BE BUILT

```
EXISTS NOW (working):
  skills/        ← TOML drop, Hydra loads on boot ✅
  genome.toml    ← situation/approach pairs ✅
  functor.toml   ← domain concept → axiom primitive mappings ✅

NEEDS TO BE BUILT (architecture ready, loader not implemented):
  integrations/  ← TOML drop for API connections
  actions/       ← TOML drop for executable actions
  credentials/   ← secure credential storage

IMPLEMENTATION ESTIMATE:
  Integration loader: ~200 lines in hydra-executor
    - Read api.toml on boot
    - Register capabilities in action registry
    - Use hydra-protocol for API calls
    - Use hydra-reach-extended for connectivity

  Action loader: ~150 lines in hydra-executor
    - Read action.toml on boot
    - Register in scheduler (if scheduled)
    - Approval gate before write actions
    - Receipt after execution

  Credential manager: ~100 lines in hydra-environment
    - Read credentials.toml
    - Never log, never persist to memory
    - Encrypt at rest (future)

  Total: ~450 lines. No changes to the kernel.
  All additive. All in existing crates.
```

---

## THE VISION

```
Today:
  skills/ folder makes Hydra knowledgeable

Tomorrow:
  integrations/ folder makes Hydra connected
  actions/ folder makes Hydra capable

Combined:
  Knowledge + Connection + Capability = Autonomy

  Hydra knows what to do (genome)
  Hydra can reach the system (integration)
  Hydra can execute the action (action)
  Hydra asks permission first (approval)
  Hydra records everything (receipt)

  No code. Just TOML files in folders.
  Anyone can extend Hydra.
  A nurse. A trader. A teacher. A farmer.
  Drop the files. Hydra learns. Hydra connects. Hydra acts.
```

---

*The brain does not change. The folders do.*
*Hydra's architecture was designed for this from day one.*
*68 crates of infrastructure so that capability is a text file.*
