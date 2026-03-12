#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$HOME/.local/bin/hydra-server"
PLIST_NAME="com.agentra.hydra.plist"
SERVICE_NAME="hydra.service"

# --- Pre-flight check ---
if [ ! -f "$BINARY" ]; then
    echo "ERROR: hydra-server binary not found at $BINARY"
    echo "Build and install it first:  cargo install --path crates/hydra-server --root ~/.local"
    exit 1
fi

echo "Found hydra-server at $BINARY"

# --- Detect OS and install ---
case "$(uname -s)" in
    Darwin)
        echo "Detected macOS — installing launchd plist..."

        DEST="$HOME/Library/LaunchAgents/$PLIST_NAME"
        SRC="$SCRIPT_DIR/$PLIST_NAME"

        # Unload existing service if present (ignore errors)
        launchctl unload "$DEST" 2>/dev/null || true

        # Rewrite plist with the current user's HOME
        sed "s|/Users/omoshola|$HOME|g" "$SRC" > "$DEST"

        launchctl load "$DEST"
        echo "Loaded $PLIST_NAME via launchctl."
        echo ""
        echo "Status:"
        launchctl list | grep com.agentra.hydra || echo "  (service not yet running — check logs)"
        echo "Logs: ~/Library/Logs/hydra-server.log"
        ;;

    Linux)
        echo "Detected Linux — installing systemd user service..."

        DEST_DIR="$HOME/.config/systemd/user"
        mkdir -p "$DEST_DIR"

        cp "$SCRIPT_DIR/$SERVICE_NAME" "$DEST_DIR/$SERVICE_NAME"

        systemctl --user daemon-reload
        systemctl --user enable "$SERVICE_NAME"
        systemctl --user start "$SERVICE_NAME"

        echo ""
        echo "Status:"
        systemctl --user status "$SERVICE_NAME" --no-pager || true
        ;;

    *)
        echo "ERROR: Unsupported OS ($(uname -s)). Only macOS and Linux are supported."
        exit 1
        ;;
esac

echo ""
echo "Hydra daemon installed successfully."
