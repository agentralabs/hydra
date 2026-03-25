#!/usr/bin/env bash
# Hydra installer — clean progress bar, no cargo spam.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
HYDRA_HOME="${HOME}/.hydra"
INSTALL_DIR="${HOME}/.local/bin"
LOG="${HYDRA_HOME}/build.log"
TOTAL_CRATES=150

echo ""
echo "  ◈ Hydra — Autonomous Digital Intelligence"
echo ""

# Check dependencies
OS="$(uname -s)"; ARCH="$(uname -m)"
printf "  Detecting platform... %s %s ✓\n" "$OS" "$ARCH"

if ! command -v cargo &>/dev/null; then
    echo "  ✗ Rust/Cargo not found. Install from https://rustup.rs"; exit 1
fi
printf "  Checking Rust toolchain... ✓\n"

if [ ! -f "${REPO_DIR}/Cargo.toml" ]; then
    echo "  ✗ Run this script from the hydra repo."; exit 1
fi

# Build with progress bar
echo ""
echo "  Building release binary (3-8 minutes)..."
echo ""
mkdir -p "$HYDRA_HOME"
: > "$LOG"

cd "$REPO_DIR"
COMPILED=0
cargo build --release -p hydra-kernel -p hydra-tui 2>&1 | while IFS= read -r line; do
    echo "$line" >> "$LOG"
    if echo "$line" | grep -q "Compiling "; then
        COMPILED=$((COMPILED + 1))
        PCT=$((COMPILED * 100 / TOTAL_CRATES))
        if [ "$PCT" -gt 100 ]; then PCT=99; fi
        CRATE=$(echo "$line" | sed 's/.*Compiling //' | sed 's/ v.*//')
        FILLED=$((PCT / 3))
        EMPTY=$((33 - FILLED))
        BAR=""
        for _ in $(seq 1 "$FILLED" 2>/dev/null); do BAR="${BAR}█"; done
        for _ in $(seq 1 "$EMPTY" 2>/dev/null); do BAR="${BAR}░"; done
        printf "\r  [%s] %3d%%  %-30s" "$BAR" "$PCT" "$CRATE"
    fi
done

printf "\r  [█████████████████████████████████] 100%%  Done                          \n"
echo ""

# Create directory structure
mkdir -p "${HYDRA_HOME}/data" "${HYDRA_HOME}/backups" "${HYDRA_HOME}/logs"

# Copy skills if not already present
if [ ! -d "${HYDRA_HOME}/skills" ] && [ -d "${REPO_DIR}/skills" ]; then
    cp -r "${REPO_DIR}/skills" "${HYDRA_HOME}/skills"
    echo "  ✓ Skills installed"
fi

# Copy integrations if not already present
if [ ! -d "${HYDRA_HOME}/integrations" ] && [ -d "${REPO_DIR}/integrations" ]; then
    cp -r "${REPO_DIR}/integrations" "${HYDRA_HOME}/integrations"
    echo "  ✓ Integrations installed"
fi

# Install binaries — just 'hydra' and 'hydra-tui'
mkdir -p "$INSTALL_DIR"
if [ -f "${REPO_DIR}/target/release/hydra" ]; then
    cp "${REPO_DIR}/target/release/hydra" "${INSTALL_DIR}/hydra"
    chmod +x "${INSTALL_DIR}/hydra"
fi
if [ -f "${REPO_DIR}/target/release/hydra_tui" ]; then
    cp "${REPO_DIR}/target/release/hydra_tui" "${INSTALL_DIR}/hydra-tui"
    chmod +x "${INSTALL_DIR}/hydra-tui"
    # Also create 'hydra tui' alias via symlink
    ln -sf "${INSTALL_DIR}/hydra-tui" "${INSTALL_DIR}/hydra_tui" 2>/dev/null || true
fi
echo "  ✓ Binaries installed to ${INSTALL_DIR}/"

# PATH check
if ! echo "$PATH" | grep -q "${INSTALL_DIR}"; then
    SHELL_RC="${HOME}/.$(basename "$SHELL")rc"
    echo "export PATH=\"\${HOME}/.local/bin:\${PATH}\"" >> "$SHELL_RC"
    echo "  ✓ PATH updated in ${SHELL_RC}"
    echo "    (restart your terminal or run: source ${SHELL_RC})"
fi

# .env setup
if [ ! -f "${REPO_DIR}/.env" ] && [ -f "${REPO_DIR}/.env.example" ]; then
    cp "${REPO_DIR}/.env.example" "${REPO_DIR}/.env"
    echo "  ✓ .env created from example (add your ANTHROPIC_API_KEY)"
fi

echo ""
echo "  ◈ Hydra installed successfully"
echo ""
echo "  Commands:"
echo "    hydra \"your question\"          Single-shot mode"
echo "    hydra --interactive             Interactive REPL"
echo "    hydra --daemon                  Always-on background daemon"
echo "    hydra-tui                       Full TUI cockpit"
echo ""
echo "  Quick start:"
echo "    1. Add your API key:  echo 'ANTHROPIC_API_KEY=sk-...' >> .env"
echo "    2. Run:               hydra \"hello\""
echo ""
echo "  Data: ${HYDRA_HOME}/"
echo ""

# Daemon install
if [ "${1:-}" = "--daemon" ]; then
    if [ "$OS" = "Darwin" ]; then bash "${REPO_DIR}/scripts/install-daemon.sh" install
    else bash "${REPO_DIR}/scripts/install-daemon-linux.sh" install; fi
fi
