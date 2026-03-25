#!/usr/bin/env bash
# Hydra Universal Installer — one command, any machine.
#
# Usage (when published):
#   curl -fsSL https://hydra.agentralabs.com/install | bash
#
# What it does:
#   1. Installs Rust if not present
#   2. Clones the repo
#   3. Builds release binaries
#   4. Installs 'hydra' and 'hydra-tui' to PATH
#   5. Creates ~/.hydra with skills and integrations
#   6. Optionally installs as daemon (--daemon flag)
#
# Works on: macOS (Intel/ARM), Linux (x86_64/aarch64)

set -euo pipefail

REPO="https://github.com/agentralabs/hydra.git"
INSTALL_DIR="${HOME}/.local/bin"
HYDRA_HOME="${HOME}/.hydra"
CLONE_DIR="${HOME}/.hydra/src"

echo ""
echo "  ◈ H Y D R A — Universal Installer"
echo "  The first autonomous digital entity."
echo ""

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"
printf "  Platform: %s %s\n" "$OS" "$ARCH"

# Install Rust if needed
if ! command -v cargo &>/dev/null; then
    echo "  Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --quiet
    source "${HOME}/.cargo/env"
    echo "  ✓ Rust installed"
else
    echo "  ✓ Rust found"
fi

# Clone or update repo
if [ -d "${CLONE_DIR}/.git" ]; then
    echo "  Updating Hydra source..."
    cd "$CLONE_DIR" && git pull --quiet
else
    echo "  Cloning Hydra..."
    mkdir -p "$(dirname "$CLONE_DIR")"
    git clone --quiet --depth 1 "$REPO" "$CLONE_DIR"
fi

# Build
cd "$CLONE_DIR"
echo ""
echo "  Building (3-8 minutes on first install)..."
cargo build --release -p hydra-kernel -p hydra-tui --quiet 2>/dev/null || \
    cargo build --release -p hydra-kernel -p hydra-tui

# Install binaries
mkdir -p "$INSTALL_DIR"
cp target/release/hydra "${INSTALL_DIR}/hydra" 2>/dev/null || true
cp target/release/hydra_tui "${INSTALL_DIR}/hydra-tui" 2>/dev/null || true
chmod +x "${INSTALL_DIR}/hydra" "${INSTALL_DIR}/hydra-tui" 2>/dev/null || true

# PATH
if ! echo "$PATH" | grep -q "${INSTALL_DIR}"; then
    SHELL_RC="${HOME}/.$(basename "${SHELL:-bash}")rc"
    echo "export PATH=\"\${HOME}/.local/bin:\${PATH}\"" >> "$SHELL_RC"
    export PATH="${INSTALL_DIR}:${PATH}"
fi

# Create data directories
mkdir -p "${HYDRA_HOME}/data" "${HYDRA_HOME}/backups" "${HYDRA_HOME}/logs"
mkdir -p "${HYDRA_HOME}/drop/processed" "${HYDRA_HOME}/drop/rejected"
mkdir -p "${HYDRA_HOME}/connectors" "${HYDRA_HOME}/vault"

# Copy skills
if [ ! -d "${HYDRA_HOME}/skills" ] && [ -d "${CLONE_DIR}/skills" ]; then
    cp -r "${CLONE_DIR}/skills" "${HYDRA_HOME}/skills"
fi

# .env
if [ ! -f "${CLONE_DIR}/.env" ] && [ -f "${CLONE_DIR}/.env.example" ]; then
    cp "${CLONE_DIR}/.env.example" "${CLONE_DIR}/.env"
fi

# Install daemon if requested
if [ "${1:-}" = "--daemon" ]; then
    if [ "$OS" = "Darwin" ]; then
        bash "${CLONE_DIR}/scripts/install-daemon.sh" install
    else
        bash "${CLONE_DIR}/scripts/install-daemon-linux.sh" install
    fi
    echo "  ✓ Daemon installed (auto-starts on boot)"
fi

echo ""
echo "  ◈ Hydra installed successfully"
echo ""
echo "  Commands:"
echo "    hydra \"hello\"              Talk to Hydra"
echo "    hydra --daemon              Run as background daemon"
echo "    hydra-tui                   Full TUI cockpit"
echo ""
echo "  First time? Add your API key:"
echo "    echo 'ANTHROPIC_API_KEY=sk-ant-...' >> ${CLONE_DIR}/.env"
echo ""
echo "  Then: hydra \"hello\""
echo ""
