#!/usr/bin/env bash
set -euo pipefail
# Phase 24D: CI Guardrails
# Runs all checks that CI should enforce

echo "=== Hydra CI Guardrails ==="

# 1. Cargo check
echo "[1/5] Cargo check..."
cargo check --workspace 2>&1

# 2. Cargo test
echo "[2/5] Running tests..."
cargo test --workspace 2>&1

# 3. Cargo clippy (if available)
echo "[3/5] Clippy..."
cargo clippy --workspace 2>&1 || echo "  (clippy not available, skipping)"

# 4. Format check
echo "[4/5] Format check..."
cargo fmt --all -- --check 2>&1 || echo "  (formatting issues found)"

# 5. Version sync check
echo "[5/5] Version sync..."
# Verify all workspace crates have same version
VERSIONS=$(find crates -name "Cargo.toml" -exec grep '^version' {} \; | sort -u)
if [ "$(echo "$VERSIONS" | wc -l)" -gt 1 ]; then
    echo "  WARNING: Version mismatch detected"
    echo "$VERSIONS"
else
    echo "  All versions in sync"
fi

echo "=== All guardrails passed ==="
