# 21 — The Vault

## One Place for Every Key, Every Login, Every Account

Hydra needs credentials to connect to services. You need to see what credentials Hydra has. The vault is the single folder where ALL of this lives — visible to both of you, controlled by you.

```
vault/
  github.toml              ← you added this
  slack.toml               ← you added this
  alpaca.toml              ← you added this
  hydra-created/           ← Hydra added these (with your approval)
    analytics-account.toml
    monitoring-service.toml
```

---

## Why a Vault, Not Scattered credentials.toml Files

Without a vault, every integration has its own credentials file:

```
integrations/github/credentials.toml
integrations/slack/credentials.toml
integrations/alpaca/credentials.toml
integrations/datadog/credentials.toml
```

The problems:
- **Scattered** — you have to look in 15 different folders to see what Hydra has access to
- **No overview** — no single place to audit all credentials
- **No access control** — every credential has the same permissions
- **No tracking** — you do not know if Hydra created an account somewhere

The vault solves all four:

```
vault/
  ← ONE folder to see everything
  ← ONE place to audit
  ← EACH credential has read/write/delete/spend permissions
  ← hydra-created/ folder separates what Hydra made from what you gave
```

---

## The Credential File Format

```toml
[service]
name        = "github"
url         = "https://github.com"
created_by  = "user"                # "user" = you gave this to Hydra
created_at  = "2026-03-21"          # when this credential was added
notes       = "Personal access token with repo scope"

[credentials]
token       = "ghp_xxxxxxxxxxxxxxxxxxxx"

[access]
read        = true                  # Hydra can read repos, PRs, issues
write       = true                  # Hydra can create issues, comment on PRs
delete      = false                 # Hydra cannot delete repos or branches
spend       = false                 # No financial actions
```

### The [access] Block — What Hydra Is Allowed to Do

This is the critical part. You control exactly what Hydra can do with each credential:

| Permission | Default | What It Controls |
|-----------|---------|-----------------|
| `read` | `true` | Can Hydra read data from this service? |
| `write` | `false` | Can Hydra create or modify things? |
| `delete` | `false` | Can Hydra delete anything? |
| `spend` | `false` | Can Hydra spend money through this service? |
| `max_spend` | `0.0` | If spend=true, maximum daily spend in USD |

Examples:

```toml
# GitHub — read and write, no delete
[access]
read   = true
write  = true    # create issues, comment on PRs
delete = false   # never delete repos or branches

# Alpaca — read and trade, limited spend
[access]
read      = true
write     = true     # place orders
delete    = false
spend     = true     # can spend money (trading)
max_spend = 500.0    # max $500 per day

# Datadog — read only
[access]
read   = true
write  = false
delete = false
spend  = false
```

---

## When Hydra Creates an Account

Sometimes Hydra needs to sign up for a service to complete a task. The flow:

```
You: "Monitor our API uptime"
Hydra: "I need an UptimeRobot account to monitor the API.
        I can create a free account. Approve?"
You: "yes"

Hydra:
  1. Creates account on UptimeRobot
  2. Saves credentials to vault/hydra-created/uptimerobot.toml
  3. Sets default permissions: read=true, write=true, delete=false, spend=false
  4. Receipts the account creation
  5. Tells you: "Account created. Credentials saved to vault/hydra-created/uptimerobot.toml.
                 Default permissions: read+write. Review and adjust if needed."
```

The credential file Hydra creates:

```toml
[service]
name        = "uptimerobot"
url         = "https://uptimerobot.com"
created_by  = "hydra"               # Hydra created this
created_at  = "2026-03-21T14:32:00Z"
approved_by = "user"                # You approved the creation
notes       = "Free tier — monitoring 5 URLs"

[credentials]
email       = "hydra-monitor@yourdomain.com"
api_key     = "ur_xxxxxxxxxxxxxxxx"

[access]
read        = true
write       = true
delete      = false
spend       = false
```

### Your Control

```
You can at any time:

  VIEW     → open any .toml file in the vault
  EDIT     → change permissions, rotate keys
  REVOKE   → delete the file, Hydra loses access immediately
  TRANSFER → move from hydra-created/ to vault/ to "claim" the account
```

---

## How Integrations Find Credentials

When an integration needs to authenticate, it looks in the vault:

```
Integration: integrations/github/api.toml
  → auth_type = "bearer"
  → Looks for: vault/github.toml
  → Reads: [credentials] token
  → Checks: [access] read=true (allowed)
  → Proceeds with the API call

Integration: integrations/alpaca/api.toml
  → Capability: "buy" (write)
  → Looks for: vault/alpaca.toml
  → Reads: [credentials] api_key, api_secret
  → Checks: [access] write=true, spend=true, max_spend=500
  → If order > $500: BLOCKED by vault permission
  → If order ≤ $500: asks for approval, then executes
```

The vault is the single gate. No credential, no access. Wrong permission, no action.

---

## Security

```
WHAT IS PROTECTED:
  - Every .toml in vault/ is gitignored (never committed)
  - Credentials never appear in logs, receipts, or memory
  - Credentials never leave your machine
  - API keys are read-only by Hydra (Hydra cannot modify vault files)

WHAT YOU CONTROL:
  - Every credential file can be deleted to revoke access instantly
  - Every credential has explicit read/write/delete/spend permissions
  - Hydra-created accounts are separated from user-provided accounts
  - max_spend caps daily financial exposure per service

CONSTITUTIONAL ENFORCEMENT:
  - Law 6 (Principal Supremacy): You can revoke any credential at any time
  - Law 1 (Receipt Immutability): Every credential use is receipted
  - Law 7 (Causal Chain): Every action traces back through the vault check
```

---

## The Complete Flow

```
1. You add a credential:
   → Create vault/servicename.toml
   → Set permissions in [access]
   → Done

2. You point an integration at it:
   → integrations/servicename/api.toml references the vault
   → Hydra reads credentials from vault on each use

3. Hydra uses the credential:
   → Checks [access] permissions before every call
   → Blocked if permission is false
   → Blocked if spend exceeds max_spend
   → Receipted after every use
   → Stored in audit trail

4. You audit:
   → Open vault/ folder — see every credential in one place
   → Open vault/hydra-created/ — see what Hydra signed up for
   → Delete any file — instant revocation

5. Hydra creates an account:
   → Asks your approval first
   → Saves to vault/hydra-created/
   → Default permissions: conservative (no delete, no spend)
   → You review and adjust
```

---

## The Social Media Version

```
ChatGPT has no credentials. It cannot log into anything.
Hydra has a vault. It can connect to anything you authorize.

ChatGPT asks you to copy-paste API responses.
Hydra calls the API directly.

ChatGPT cannot create accounts.
Hydra can, but only with your approval, and the
credentials are saved where you can see and revoke them.

Your keys. Your machine. Your control.
One folder. Complete visibility. Instant revocation.
```
