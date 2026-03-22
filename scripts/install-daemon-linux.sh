#!/bin/bash
# Install Hydra as a Linux daemon (systemd service)
# Hydra will start at boot and run continuously.
#
# Usage:
#   bash scripts/install-daemon-linux.sh             # install and start
#   bash scripts/install-daemon-linux.sh stop         # stop
#   bash scripts/install-daemon-linux.sh uninstall    # remove

set -e

HYDRA_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SERVICE_NAME="hydra"
SERVICE_FILE="/etc/systemd/system/$SERVICE_NAME.service"
LOG_DIR="$HOME/.hydra/logs"
HYDRA_BIN="$HYDRA_DIR/target/release/hydra"
USER=$(whoami)

case "${1:-install}" in
  install)
    echo "Building Hydra (release mode)..."
    cargo build -p hydra-kernel --bin hydra --release

    echo "Creating log directory..."
    mkdir -p "$LOG_DIR"

    echo "Creating systemd service..."
    sudo tee "$SERVICE_FILE" > /dev/null << EOF
[Unit]
Description=Hydra — Agentra Labs Autonomous Entity
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$HYDRA_DIR
ExecStart=$HYDRA_BIN --daemon
Restart=always
RestartSec=5
Environment=HYDRA_MODE=daemon
Environment=HYDRA_LOG=info
StandardOutput=append:$LOG_DIR/hydra.stdout.log
StandardError=append:$LOG_DIR/hydra.stderr.log

# Resource limits
Nice=10
LimitNOFILE=65536
MemoryMax=2G

[Install]
WantedBy=multi-user.target
EOF

    echo "Enabling and starting..."
    sudo systemctl daemon-reload
    sudo systemctl enable "$SERVICE_NAME"
    sudo systemctl start "$SERVICE_NAME"

    echo ""
    echo "Hydra daemon installed and running."
    echo "  Status: sudo systemctl status hydra"
    echo "  Logs:   journalctl -u hydra -f"
    echo "  Stop:   bash scripts/install-daemon-linux.sh stop"
    echo ""
    echo "Hydra is now always on."
    ;;

  stop)
    echo "Stopping Hydra daemon..."
    sudo systemctl stop "$SERVICE_NAME"
    echo "Stopped."
    ;;

  start)
    echo "Starting Hydra daemon..."
    sudo systemctl start "$SERVICE_NAME"
    echo "Started. Logs: journalctl -u hydra -f"
    ;;

  uninstall)
    echo "Uninstalling Hydra daemon..."
    sudo systemctl stop "$SERVICE_NAME" 2>/dev/null || true
    sudo systemctl disable "$SERVICE_NAME" 2>/dev/null || true
    sudo rm -f "$SERVICE_FILE"
    sudo systemctl daemon-reload
    echo "Uninstalled."
    ;;

  status)
    sudo systemctl status "$SERVICE_NAME" --no-pager
    ;;

  *)
    echo "Usage: $0 {install|start|stop|uninstall|status}"
    exit 1
    ;;
esac
