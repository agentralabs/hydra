# HYDRA VAULT

## What This Folder Is

This is where ALL credentials live — every login, every API key, every account Hydra uses or creates. One folder. One place. Both you and Hydra can see everything here.

## Rules

1. **Everything in this folder is gitignored** — never committed
2. **One file per service** — named clearly so you can find it
3. **Hydra reads from here** — integrations and actions look here for credentials
4. **Hydra writes here** — when Hydra creates an account, credentials go here
5. **You can edit any file** — rotate keys, update passwords, revoke access
6. **Deleting a file revokes access** — Hydra can no longer reach that service

## Structure

```
vault/
  README.md           ← this file
  github.toml         ← GitHub credentials
  slack.toml          ← Slack credentials
  alpaca.toml         ← Brokerage credentials
  openweather.toml    ← Weather API key
  stripe.toml         ← Stripe credentials
  twitter.toml        ← Twitter/X credentials
  hydra-created/      ← accounts Hydra created on its own
    service-xyz.toml
```
