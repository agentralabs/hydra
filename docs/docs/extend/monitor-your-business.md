# Monitor Your Business with Hydra

Hydra can act as your cybersecurity expert, operations manager, database administrator, and site reliability engineer — all at once. You teach it by dropping TOML files.

## Quick Setup (5 minutes)

### Step 1: Add your site as a connector

```bash
# Create the connector file
cat > ~/.hydra/connectors/my-site.toml << 'EOF'
[connector]
name        = "my-site"
type        = "http"
url         = "https://www.yoursite.com"
method      = "GET"
interval    = 60
expect_status = 200
timeout     = 10

[alerts]
on_failure  = "Site down! yoursite.com returned non-200"
on_slow     = "Slow response from yoursite.com (>5s)"
slow_threshold_ms = 5000
EOF
```

That's it. Hydra's monitor hub (O16) picks this up automatically and starts polling every 60 seconds.

### Step 2: Add your server for SSH access

```bash
cat >> ~/.hydra/machines.toml << 'EOF'
[[machines]]
name = "production"
host = "your-server.com"
user = "deploy"
port = 22
EOF
```

Now Hydra can SSH into your server to check logs, restart services, and diagnose issues.

### Step 3: Drop the monitoring skills

The skills that ship with Hydra handle the rest:

| Skill | What It Does | How to Use |
|---|---|---|
| `site-monitor` | SSL, DNS, headers, response time, port scan, uptime | "check SSL for yoursite.com" |
| `security-ops` | Vulnerability audit, exposed files, CORS, WAF detection | "security audit https://yoursite.com" |
| `database-ops` | Connection check, slow queries, table sizes, backup verify | "check database on production" |
| `ops-manager` | Server health, processes, logs, Docker, nginx, disk cleanup | "server health on production" |

These are already in the `skills/` directory. Hydra loads them on startup.

## What Hydra Does Automatically

Once configured, Hydra's daemon mode runs these checks continuously:

| Check | Frequency | Alert |
|---|---|---|
| HTTP uptime | Every 60s | Immediate alert if status != 200 |
| SSL certificate | Daily (dream loop) | Alert if expiry < 30 days |
| Response time | Every 60s | Alert if > 5 seconds |
| Content hash | Every 5 minutes | Alert if page content changed (defacement) |
| DNS resolution | Hourly | Alert if DNS fails |

### Start the daemon:

```bash
# Interactive (see the monitoring in real-time)
cargo run -p hydra-tui --bin hydra_tui

# Background daemon (runs 24/7, survives reboots)
cargo run -p hydra-kernel --bin hydra -- --daemon
```

## Talking to Hydra About Your Business

In the TUI, just ask naturally:

```
you > check the SSL certificate for zexrail.com
you > is the production server healthy?
you > any slow queries on the database?
you > run a security audit on https://www.zexrail.com
you > what ports are open on production?
you > check the nginx status on production
you > are there any errors in the application logs?
```

Hydra uses the skills you dropped + the servers you configured + its own genome knowledge to answer. Over time, it learns your infrastructure's patterns and gets better at predicting issues.

## Custom Monitoring Skill

Want Hydra to check something specific to your business? Create a skill:

```bash
mkdir skills/my-business-checks
```

```toml
# skills/my-business-checks/operations.toml

[[operations]]
name        = "check-payment-gateway"
trigger     = "payment gateway|stripe status|payment check"
params      = []
confidence  = 0.90
steps       = [
    "curl -s https://status.stripe.com/api/v2/summary.json | python3 -c \"import sys,json; d=json.load(sys.stdin); print(d.get('status',{}).get('description','unknown'))\"",
]
description = "Check Stripe payment gateway status"

[[operations]]
name        = "check-email-deliverability"
trigger     = "email status|email deliverability|mail check"
params      = ["domain"]
confidence  = 0.85
steps       = [
    "dig TXT {domain} | grep -i spf",
    "dig TXT _dmarc.{domain} | grep -i dmarc",
]
description = "Check SPF and DMARC records for email deliverability"
```

Restart Hydra. Now you can say: "check the payment gateway" and Hydra knows what to do.

## Database Credentials

Store database credentials securely in the vault:

```bash
cat > vault/database.toml << 'EOF'
[service]
name = "production-db"
url  = "postgresql://your-server:5432/mydb"

[credentials]
username = "hydra_readonly"
password = "your-secure-password"

[access]
read   = true
write  = false
delete = false
EOF
```

Hydra reads this via the vault system (AES-256-GCM encryption if `HYDRA_VAULT_PASSPHRASE` is set).

## Proactive Monitoring

With the daemon running, Hydra doesn't just respond to your questions. It **proactively detects and alerts**:

- Site goes down → Hydra notices within 60 seconds and logs an alert
- SSL expiring → Hydra warns you 30 days before
- Error rate spikes → Hydra checks logs and reports what changed
- Disk filling up → Hydra suggests cleanup before it hits 90%

The proactive engine (O31) evaluates triggers against the autonomy gradient (O29). High-confidence, low-blast actions (like checking logs) happen automatically. High-risk actions (like restarting services) ask for your approval first.

## Multiple Sites

Repeat the connector setup for each site:

```bash
# Site 2
cat > ~/.hydra/connectors/api-server.toml << 'EOF'
[connector]
name = "api-server"
type = "http"
url  = "https://api.yoursite.com/health"
interval = 30
expect_status = 200
EOF

# Site 3
cat > ~/.hydra/connectors/staging.toml << 'EOF'
[connector]
name = "staging"
type = "http"
url  = "https://staging.yoursite.com"
interval = 120
expect_status = 200
EOF
```

Hydra monitors them all concurrently. Each site has independent alerting.

## What Hydra Learns Over Time

| Day 1 | Day 30 | Day 90 |
|---|---|---|
| Runs your monitoring commands | Knows your server's normal CPU baseline | Predicts issues before they happen |
| Alerts on failures | Recognizes recurring Tuesday deploys | Auto-suggests pre-deploy checks |
| Checks what you ask | Knows which errors are noise vs critical | Filters noise, escalates real issues |

The genome accumulates your infrastructure's patterns. After 90 days, Hydra knows your systems better than any dashboard.
