# Sister Release Gate Checklist
## Every box must be checked before publishing to crates.io / PyPI / npm.

### Engineering (run `cargo check && cargo test && cargo clippy`)
- [ ] `cargo check -p <sister>` — zero errors
- [ ] `cargo test -p <sister>` — zero failures
- [ ] `cargo clippy -p <sister> -- -D warnings` — zero warnings
- [ ] No file in `src/` exceeds 400 lines
- [ ] No hardcoded values — all constants in `constants.rs`
- [ ] No `unwrap()` in library code (only in tests)

### MCP Hardening
- [ ] All MCP tool handlers validate input — no silent fallback on missing params
- [ ] Canonical-path hashing for project identity (same name = same project)
- [ ] Concurrent startup lock with stale-lock recovery
- [ ] Merge-only MCP config update (never overwrites existing entries)
- [ ] Server mode: `SISTER_AUTH_TOKEN` environment variable gates access

### Installer
- [ ] Profile-based install: `--profile desktop|terminal|server`
- [ ] Post-install message explicitly says "restart your MCP client"
- [ ] Optional feedback prompt at end of install

### Bridge Compliance
- [ ] `bridges.rs` present with all 6 traits implemented
- [ ] `NoOpBridges` for standalone mode
- [ ] `HydraAdapter` for orchestration mode
- [ ] Bridge tests pass (minimum 6 bridge tests)

### Stress Tests
- [ ] Multi-project isolation: same folder name, different paths -> different identities
- [ ] Concurrent startup: 3 simultaneous starts -> only one succeeds, others fail gracefully
- [ ] Restart continuity: data survives kill + reboot
- [ ] Server auth: unauthenticated request rejected; authenticated accepted

### Publishing
- [ ] `README.md` with install instructions and quick-start
- [ ] `CHANGELOG.md` entry for this release version
- [ ] Version in `Cargo.toml` matches changelog entry
- [ ] `ci/check-sister-hardening.sh crates/<sister>` exits 0

### Final Gate
- [ ] All boxes above are checked
- [ ] `ci/check-sister-hardening.sh` exits 0
- [ ] At least one human has read the new code

**DO NOT PUBLISH if any box is unchecked. No exceptions.**
