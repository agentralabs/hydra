---

## MANDATORY HARDENING — ALL SISTERS BUILT AFTER MARCH 2026

Every sister shipped after this date must pass all 10 checks
before being considered release-ready:

1. Strict MCP parameter validation — no silent fallbacks on missing params
2. Canonical-path hashed identity per project (same folder name ≠ same identity)
3. Zero cross-project contamination (no "latest cached" fallback across projects)
4. Concurrent startup locking with stale-lock recovery
5. Merge-only MCP config updates — never destructive overwrite
6. Profile-based universal installer (desktop | terminal | server)
7. Explicit post-install restart guidance in installer output
8. Optional feedback prompt at end of install
9. Server-mode auth gating via SISTER_AUTH_TOKEN environment variable
10. Automated regression tests: multi-project, concurrent startup, restart continuity

The CI gate (ci/check-sister-hardening.sh) enforces checks 1, 2, 4, 5, 6, 9
automatically on every PR. Checks 3, 7, 8, 10 are verified in the
release gate checklist (ci/SISTER-RELEASE-GATE.md).

A sister that does not pass all 10 is NOT release-ready.
No exceptions. No deferrals.
