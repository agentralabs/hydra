# O25: Hardening Mega-Session

**Status:** Complete
**Session:** 32
**Built:** 2026-03-24

## What It Does
Cross-cutting quality and security hardening across ALL 29 original orchestrations. Not a new capability — a systematic audit that found and fixed 33 edge cases, wired dead code, added spawn safety, and closed security vulnerabilities. Every orchestration is now runtime-verified and production-ready.

## Files Modified (20+)
- conductor_exec.rs — Shell timeout enforcement (60s killpg), task timeout (10min), convergence detection
- zero_defect.rs — Gate timeout (30s killpg), removed cargo fallbacks
- parallel.rs — Shell concurrency limit, cascade failure cancellation
- assumptions.rs — Sort by severity, dangerous ops detection, local timezone
- conductor.rs — reqwest 15s timeout, remote step type, visible skip for unrecognized steps
- worker.rs — URL shell escaping, trust from genome, honest interface messages
- workspace.rs — Port remapping persisted in restart command
- monitor/mod.rs — Event buffer cap (500), auto-action timeout (10s killpg), connector loading
- collaboration.rs — FileObserver auto-enabled
- social.rs — Expanded sarcasm markers
- fingerprint.rs — Expanded stealth JS (screen dimensions, device memory)
- engine.rs — Rate limiter domain wiring, stealth JS injection
- http_api.rs — Localhost binding, perceive pipeline for /api/cycle
- remote.rs — Credential redaction, stronger 6-digit PIN
- remote_exec.rs — StrictHostKeyChecking=accept-new, command shell escaping
- security.rs — Enforced blocks (neutralize input), real feature extraction
- prompt.rs — Credential redaction before LLM, prompt boundaries, skill creation hint
- vault_crypto.rs — File permissions 0o600
- backup.rs — Minimum 2 backups before prune
- screen.rs — Retina scale factor, uniform image detection
- ocr.rs — Dark mode OCR retry with image inversion
- store.rs — Genome eviction when full (lowest confidence + use)
- operations.rs — Destructive command blocking
- critic.rs — Weighted score clamping, visual/style evaluation functions
- observer.rs — Credential files in ignore list
- document.rs — 50MB file size limit
- hydra_tui_v2.rs — boot_systems, shutdown_systems, ambient loop, voice stop, tokio shutdown, idle tracking, first-run wizard before raw mode
- tui_helpers.rs — ConductorUpdate::Info variant, boot health check
- loop_dream.rs — Swarm idle_secs from DreamSubsystems, backup_merge inbox
- loop_ambient.rs — Cloud backup wiring, drop gateway wiring, evolution wiring
- immersion/mod.rs — Survey web search auto-fetch
- evolution/writer.rs — Self-drop audit trail
- learn_md.rs — 10 compliance fixes (path sanitization, file size, merge, constants, etc.)
- growth.rs — Rich output preference to genome

## Dependencies
- Depends on: ALL orchestrations O00-O24
- Required by: Production readiness

## Wiring (Law 10)
- Every fix verified: called from outside its file, reaches TUI, feeds genome
- 351+ kernel tests passing, 62/62 harness

## Edge Cases Fixed (33 total)
Bible: Sort assumptions by severity, dangerous ops → Complex, reqwest timeout, visible skip, task timeout, Retina scale, dark mode OCR, locked screen, genome eviction, long-running timeout 300s, destructive blocking, weighted score clamp, convergence detection, shell concurrency, cascade cancel, cargo fallback removal, stealth JS expansion, rate limiter wiring, sarcasm markers, survey fetch, localhost binding, credential redaction, stronger PIN, tar symlink protection, vault passphrase block, file permissions, min 2 backups, document size limit, SSH accept-new, command escaping, event buffer cap, credential file ignoring, immersion auto-fetch

## Key Decisions
- Spawn safety pattern: spawn + watchdog thread + recv_timeout + killpg — applied consistently to all shell execution (conductor 60s, zero-defect 30s, monitor 10s)
- Security middleware enforcement: blocked threats neutralize input (not just log)
- Credential never reaches LLM: redaction runs on ALL enrichments before prompt assembly
- First-run wizard BEFORE raw mode: stdin works correctly for interactive setup
