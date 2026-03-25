# O23: Universal Drop Gateway

**Status:** Complete
**Session:** 30
**Built:** 2026-03-24

## What It Does
Single entry point for ALL external items entering Hydra. Users drop files into `~/.hydra/drop/` — credentials, skills, configs, documents, connectors — and the gateway auto-classifies, validates, processes, and audits each item. Extensible via the DropHandler trait for future item types.

## Crates Used
- hydra-kernel/src/drop/mod.rs (312 lines) — Gateway orchestrator, audit trail, self-drop
- hydra-kernel/src/drop/classifier.rs (231 lines) — 15 item types, content sniffing, security checks
- hydra-kernel/src/drop/handlers.rs (341 lines) — 7 built-in handlers + extensible trait

## Dependencies
- Depends on: vault_crypto (credential encryption), learn_md (skill parsing), backup_merge (genome merge)
- Required by: O24 (connectors use drop to register)

## Wiring (Law 10)
- Called from: loop_ambient.rs (ambient tick every 5s)
- TUI visible: `/drop` command shows recent activity + audit
- Genome feedback: SkillHandler calls genome.load_from_skills() after creating skills

## Edge Cases
- File too large (>10MB) — HANDLED: size check before processing
- Path traversal in filename — HANDLED: rejects `..` and absolute paths
- Symlink in drop folder — HANDLED: rejects via symlink_metadata check
- Executable file dropped — HANDLED: ELF/Mach-O header detection
- Same file dropped twice — HANDLED: SHA256 dedup against recent audit
- Vault passphrase override via .env — HANDLED: blocks HYDRA_VAULT_PASSPHRASE key
- Unknown file type — HANDLED: moved to rejected/ with .error sidecar

## Key Decisions
- One folder (not inbox/outbox/staging) — simplicity over flexibility
- File-based polling (not inotify) — cross-platform, same pattern as FileObserver
- Self-drop capability: any module can call `drop::self_drop_file()` to create items
- Audit is append-only JSONL — immutable, forensic-ready
