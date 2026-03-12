# New Sister Release Gate Checklist

A sister is NOT release-ready until every item below is checked.
This checklist runs before `cargo publish` and before any public announcement.

## GATE 1: Canonical Structure
- [ ] docs/ecosystem/CANONICAL_SISTER_KIT.md is byte-identical to agentic-memory version
- [ ] docs/public/command-surface.md documents ALL MCP tools
- [ ] docs/public/SCENARIOS-AGENTIC-<KEY>.md exists with 5+ scenarios
- [ ] All 12 required docs/public/*.md files present
- [ ] assets/ — 4 SVGs present (Agentra design system)
- [ ] paper/paper-i-<topic>/ — .tex + references.bib present

## GATE 2: Install Contract
- [ ] scripts/install.sh supports desktop | terminal | server profiles
- [ ] MCP config writes are merge-only (tested on machine with existing config)
- [ ] Post-install restart guidance is visible in completion block
- [ ] Optional feedback prompt present

## GATE 3: MCP Quality
- [ ] All tools: verb-first descriptions, no trailing periods
- [ ] All tools: `isError: true` on execution failures (not JSON-RPC errors)
- [ ] Unknown tool returns -32803 (not -32601 or -32602)
- [ ] All parameter descriptions include defaults and valid ranges

## GATE 4: Hardening Compliance (§12)
- [ ] §12.1 Strict MCP input validation — no silent fallbacks
- [ ] §12.2 Canonical-path hashed identity/cache per project
- [ ] §12.3 Safe artifact resolution — no unrelated-cache fallback
- [ ] §12.4 PID-based lock + stale-lock recovery
- [ ] §12.5 Merge-only MCP config (tested)
- [ ] §12.6 All three installer profiles pass functional test
- [ ] §12.7 Restart guidance in completion block
- [ ] §12.8 Optional feedback prompt
- [ ] §12.9 Server mode requires AGENTIC_TOKEN
- [ ] §12.10 All four stress-tests pass:
  - [ ] Multi-project isolation
  - [ ] Concurrent startup blocking
  - [ ] Restart continuity
  - [ ] Server auth gate

## GATE 5: Test Coverage
- [ ] Hardening compliance script passes (check-hardening-compliance.sh)
- [ ] Canonical sister guardrail passes (check-canonical-sister.sh)
- [ ] cargo test --workspace passes (0 failures)
- [ ] Stress-test suite passes in all three environments (local, desktop MCP, server)

## GATE 6: CI
- [ ] All CI workflows green on main branch
- [ ] hardening-guardrails.yml workflow present and passing
- [ ] install-command-guardrails.yml passing
- [ ] canonical-sister-guardrails.yml passing

## GATE 7: Documentation
- [ ] README canonical layout with all required sections and badges
- [ ] Web docs wiring complete (discoverable from agentralabs.tech)
- [ ] Standalone guarantee present in README

---

**Approver sign-off:**
Checklist completed by: ___________
Date: ___________
Sister version: ___________
