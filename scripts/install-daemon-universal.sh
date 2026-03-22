#!/bin/bash
# Universal Hydra daemon installer — detects OS and uses the right method.
#
# Usage:
#   bash scripts/install-daemon-universal.sh             # install and start
#   bash scripts/install-daemon-universal.sh stop         # stop
#   bash scripts/install-daemon-universal.sh uninstall    # remove
#   bash scripts/install-daemon-universal.sh status       # check

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

case "$(uname -s)" in
  Darwin)
    echo "Detected: macOS — using launchd"
    bash "$SCRIPT_DIR/install-daemon.sh" "${1:-install}"
    ;;
  Linux)
    echo "Detected: Linux — using systemd"
    bash "$SCRIPT_DIR/install-daemon-linux.sh" "${1:-install}"
    ;;
  *)
    echo "Unsupported OS: $(uname -s)"
    echo "Hydra daemon requires macOS (launchd) or Linux (systemd)."
    echo "On other systems, run manually: cargo run --release -p hydra-kernel --bin hydra -- --daemon"
    exit 1
    ;;
esac
