#!/usr/bin/env bash
# Hydra binary installer — download pre-built release, no Rust needed.
# Usage: curl -fsSL https://raw.githubusercontent.com/agentralabs-tech/hydra/main/scripts/install-binary.sh | bash
set -euo pipefail

VERSION="${1:-latest}"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in x86_64) ARCH="x86_64" ;; aarch64|arm64) ARCH="aarch64" ;; esac

INSTALL_DIR="${HOME}/.local/bin"
HYDRA_HOME="${HOME}/.hydra"

echo ""
echo "  ◈ Hydra Binary Installer"
echo ""
echo "  Platform: ${OS} ${ARCH}"
echo "  Version:  ${VERSION}"
echo ""

# Download
REPO="agentralabs-tech/hydra"
if [ "$VERSION" = "latest" ]; then
    URL="https://github.com/${REPO}/releases/latest/download/hydra-${OS}-${ARCH}.tar.gz"
else
    URL="https://github.com/${REPO}/releases/download/${VERSION}/hydra-${OS}-${ARCH}.tar.gz"
fi

echo "  Downloading..."
TMP=$(mktemp -d)
if ! curl -# -L "$URL" -o "${TMP}/hydra.tar.gz" 2>&1; then
    echo "  ✗ Download failed. Check https://github.com/${REPO}/releases"
    rm -rf "$TMP"
    exit 1
fi

# Extract
echo "  Extracting..."
mkdir -p "$INSTALL_DIR" "$HYDRA_HOME"
tar xzf "${TMP}/hydra.tar.gz" -C "$INSTALL_DIR/"
rm -rf "$TMP"
chmod +x "${INSTALL_DIR}/hydra" "${INSTALL_DIR}/hydra_tui" 2>/dev/null || true

# PATH
if ! echo "$PATH" | grep -q "${INSTALL_DIR}"; then
    SHELL_RC="${HOME}/.$(basename "${SHELL:-bash}")rc"
    echo "export PATH=\"\${HOME}/.local/bin:\${PATH}\"" >> "$SHELL_RC"
    echo "  ✓ PATH updated in ${SHELL_RC}"
fi

echo ""
echo "  ✓ Hydra installed to ${INSTALL_DIR}/"
echo ""
echo "  Run:  hydra-tui"
echo ""
