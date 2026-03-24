#!/usr/bin/env bash
# Hydra universal installer — detects OS/arch, builds from source, creates ~/.hydra.
set -euo pipefail

echo "=== Hydra Installer ==="
echo ""

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"
echo "Platform: ${OS} ${ARCH}"

# Check dependencies
if ! command -v cargo &>/dev/null; then
    echo "Error: Rust/Cargo not found. Install from https://rustup.rs"
    exit 1
fi

if ! command -v git &>/dev/null; then
    echo "Error: git not found. Install git first."
    exit 1
fi

# Find repo root (or clone)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

if [ ! -f "${REPO_DIR}/Cargo.toml" ]; then
    echo "Error: Run this script from the hydra repo."
    exit 1
fi

echo ""
echo "Building Hydra (release mode)..."
cd "$REPO_DIR"

# Build main binaries
cargo build --release -p hydra-kernel -p hydra-tui 2>&1

echo ""
echo "Creating directory structure..."

HYDRA_HOME="${HOME}/.hydra"
mkdir -p "${HYDRA_HOME}/data"
mkdir -p "${HYDRA_HOME}/backups"
mkdir -p "${HYDRA_HOME}/logs"

# Copy skills if not already present
if [ ! -d "${HYDRA_HOME}/skills" ]; then
    cp -r "${REPO_DIR}/skills" "${HYDRA_HOME}/skills"
    echo "Copied default skills to ${HYDRA_HOME}/skills"
fi

# Copy integrations if not already present
if [ ! -d "${HYDRA_HOME}/integrations" ]; then
    cp -r "${REPO_DIR}/integrations" "${HYDRA_HOME}/integrations"
    echo "Copied integrations to ${HYDRA_HOME}/integrations"
fi

# Install binaries
INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"

cp "${REPO_DIR}/target/release/hydra" "${INSTALL_DIR}/hydra"
cp "${REPO_DIR}/target/release/hydra_tui" "${INSTALL_DIR}/hydra-tui"
chmod +x "${INSTALL_DIR}/hydra" "${INSTALL_DIR}/hydra-tui"

echo ""
echo "=== Installation Complete ==="
echo ""
echo "Binaries installed to: ${INSTALL_DIR}/"
echo "  hydra      — CLI and daemon"
echo "  hydra-tui  — Terminal cockpit"
echo ""
echo "Data directory: ${HYDRA_HOME}/"
echo ""

# Check if install dir is in PATH
if ! echo "$PATH" | grep -q "${INSTALL_DIR}"; then
    echo "NOTE: Add ${INSTALL_DIR} to your PATH:"
    echo "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
    echo ""
fi

# Offer daemon install
if [ "${1:-}" = "--daemon" ]; then
    echo "Installing daemon..."
    if [ "$OS" = "Darwin" ]; then
        bash "${REPO_DIR}/scripts/install-daemon.sh" install
    else
        bash "${REPO_DIR}/scripts/install-daemon-linux.sh" install
    fi
fi

echo "Run 'hydra-tui' to start the cockpit."
echo "Run 'hydra --daemon' to start the background daemon."
