# O24: Account Connectors + Active Security Monitoring

**Status:** Complete
**Session:** 31
**Built:** 2026-03-24

## What It Does
Connects Hydra to external databases, APIs, and cloud accounts for continuous security monitoring. Users drop connector configs via the gateway. Hydra polls connected accounts for threats (SQL injection, unauthorized access, credential leaks) and alerts in real-time. Also activates the immune system with real 8-dimensional threat feature extraction — replacing the empty vectors that made the immune system blind.

## Crates Used
- hydra-kernel/src/monitor/connectors.rs (268 lines) — Database/API/Cloud pollers
- hydra-kernel/src/security/features.rs (187 lines) — 8D threat feature extraction
- hydra-kernel/src/security/mod.rs (5 lines) — Module registration

## Dependencies
- Depends on: O16 (Monitor), O23 (Drop Gateway), vault_crypto (credentials)
- Required by: Future security orchestrations (S1-S6)

## Wiring (Law 10)
- Called from: MonitorMiddleware loads connectors on init, polls in post_perceive
- TUI visible: Security alerts appear as MonitorEvent(Alert) in enrichments
- Genome feedback: Immune system creates antibodies from real threat features

## Edge Cases
- Database unreachable — HANDLED: consecutive failure backoff (interval doubles)
- Query timeout — HANDLED: 10s shell timeout with killpg
- Credential not in vault — HANDLED: returns error, logs to stderr
- Connector TOML malformed — HANDLED: rejected by drop gateway with .error sidecar
- False positive on legitimate query — MITIGATED: threshold 0.5 on feature classification

## Key Decisions
- Shell-based database access (psql, mysql CLI) — no new Rust driver crates needed
- Feature extraction is structural (entropy, pattern density) not keyword matching — CLAUDE.md compliant
- Connectors stored in ~/.hydra/connectors/ — loaded on MonitorMiddleware init
- Security features: 8 dimensions (entropy, SQL, shell, credentials, priv-esc, prompt injection, size, encoding)
