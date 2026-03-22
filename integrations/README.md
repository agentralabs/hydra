# HYDRA INTEGRATIONS

## What This Folder Is

This is where Hydra connects to external services — every API, every data source, every web service. Drop a TOML file and Hydra can query a new API. No code required.

## Rules

1. **One folder per integration** — named after the service (e.g., `github-knowledge/`, `web-search/`)
2. **`api.toml` is required** — defines endpoints, auth type, and capabilities
3. **Credentials go in `vault/`** — never in this folder
4. **Read-only by default** — write capabilities must be explicitly declared
5. **Hydra loads on boot** — integrations are available immediately

## Structure

```
integrations/
  README.md                  ← this file
  github-knowledge/
    api.toml                 ← 5 read endpoints (repos, code, issues)
  web-search/
    api.toml                 ← 2 read endpoints (search, news)
  wikipedia/
    api.toml                 ← read endpoint
  youtube/
    api.toml                 ← read endpoint
  example-weather/
    api.toml                 ← example with read + write
```

## Format

```toml
# api.toml
[integration]
name = "github-knowledge"
base_url = "https://api.github.com"
auth_type = "bearer"
credential_key = "github"      # looks up vault/github.toml
documentation = "https://docs.github.com/en/rest"

[[capabilities.read]]
name = "search-repos"
endpoint = "/search/repositories?q={query}"
description = "Search GitHub repositories"

[[capabilities.read]]
name = "get-readme"
endpoint = "/repos/{owner}/{repo}/readme"
description = "Get repository README"
```

## How It Works

When Hydra needs external information that isn't in its genome or memory, it checks the integration registry. Each integration defines read and write capabilities with parameterized endpoints. Credentials are resolved from the vault at call time — never stored in the integration definition.

Drop a TOML file. Hydra connects to a new service. That is it.
