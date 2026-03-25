#!/usr/bin/env bash
# Launch Hydra TUI — clean screen, no compiler noise.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
LOG="${HOME}/.hydra/logs/build.log"
mkdir -p "$(dirname "$LOG")"

# Build quietly first (warnings go to log, not screen)
cargo build --release -p hydra-tui --bin hydra-tui --manifest-path "${REPO_DIR}/Cargo.toml" 2>"$LOG"

# Run with clean screen (stderr to log)
exec "${REPO_DIR}/target/release/hydra-tui" 2>>"${HOME}/.hydra/logs/hydra-tui.log"
