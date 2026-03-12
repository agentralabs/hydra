# SPEC: /version Slash Command

## Requirement
Add a `/version` slash command to Hydra that prints:
- Hydra version (from Cargo.toml)
- Number of connected sisters
- Current autonomy level

## Acceptance Criteria
1. User types `/version` in TUI or Desktop
2. Hydra responds with version info (not via LLM — direct handler)
3. Response includes: version string, sister count, autonomy level
4. Response appears in < 100ms (no LLM call)

## Implementation Location
- Slash command handler in hydra-native-cognitive (shared logic)
- Specifically: add a `/version` arm to `handle_universal_slash_command()` in `crates/hydra-native-cognitive/src/cognitive/handlers/llm_helpers_commands.rs`
