#!/bin/bash
# Install Hydra as a macOS daemon (launchd service)
# Hydra will start at boot and run continuously.
#
# Usage:
#   bash scripts/install-daemon.sh     # install and start
#   bash scripts/install-daemon.sh stop   # stop the daemon
#   bash scripts/install-daemon.sh uninstall  # remove completely

set -e

PLIST_NAME="com.agentra.hydra"
PLIST_SRC="$(cd "$(dirname "$0")/.." && pwd)/com.agentra.hydra.plist"
PLIST_DST="$HOME/Library/LaunchAgents/$PLIST_NAME.plist"
LOG_DIR="$HOME/.hydra/logs"

case "${1:-install}" in
  install)
    echo "Building Hydra (release mode)..."
    cargo build -p hydra-kernel --bin hydra --release

    echo "Creating log directory..."
    mkdir -p "$LOG_DIR"

    echo "Installing launchd plist..."
    cp "$PLIST_SRC" "$PLIST_DST"

    echo "Loading daemon..."
    launchctl load "$PLIST_DST" 2>/dev/null || true
    launchctl start "$PLIST_NAME" 2>/dev/null || true

    echo ""
    echo "Hydra daemon installed and running."
    echo "  Logs: $LOG_DIR/hydra.stdout.log"
    echo "  Stop: bash scripts/install-daemon.sh stop"
    echo "  Uninstall: bash scripts/install-daemon.sh uninstall"
    echo ""
    echo "Hydra is now always on."
    ;;

  stop)
    echo "Stopping Hydra daemon..."
    launchctl stop "$PLIST_NAME" 2>/dev/null || true
    echo "Stopped."
    ;;

  start)
    echo "Starting Hydra daemon..."
    launchctl start "$PLIST_NAME" 2>/dev/null || true
    echo "Started. Logs: $LOG_DIR/"
    ;;

  uninstall)
    echo "Uninstalling Hydra daemon..."
    launchctl stop "$PLIST_NAME" 2>/dev/null || true
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    rm -f "$PLIST_DST"
    echo "Uninstalled. Hydra will no longer start at boot."
    ;;

  status)
    if launchctl list | grep -q "$PLIST_NAME"; then
      echo "Hydra daemon: RUNNING"
      PID=$(launchctl list "$PLIST_NAME" 2>/dev/null | head -1 | awk '{print $1}')
      echo "  PID: $PID"
      echo "  Uptime: $(ps -p "$PID" -o etime= 2>/dev/null || echo 'unknown')"
    else
      echo "Hydra daemon: NOT RUNNING"
    fi
    ;;

  *)
    echo "Usage: $0 {install|start|stop|uninstall|status}"
    exit 1
    ;;
esac
