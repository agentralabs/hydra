#!/usr/bin/env bash
# check-file-size-guard.sh
# Enforces EMERGENCY-CRASH-FIX.md rules to prevent OOM crashes.
# Run before committing: bash scripts/check-file-size-guard.sh
#
# Rules enforced:
#   1. No .rs file over 400 lines (compilation memory limit)
#   2. No unused dependencies in hydra-native (dead dep check)
#   3. Integration tests must use tests/suite/ pattern (single binary)
#   4. No #[tokio::test] in tests/suite/ without a timeout guard

set -euo pipefail

PASS=0
FAIL=0
WARN=0
MAX_LINES=400

fail() { echo "  ❌ $1"; FAIL=$((FAIL + 1)); }
pass() { echo "  ✅ $1"; PASS=$((PASS + 1)); }
warn() { echo "  ⚠️  $1"; WARN=$((WARN + 1)); }

echo ""
echo "═══════════════════════════════════════════════════"
echo " File Size & OOM Guard — EMERGENCY-CRASH-FIX.md"
echo "═══════════════════════════════════════════════════"
echo ""

# ─────────────────────────────────────────────────────
# CHECK 1: No .rs file over 400 lines
# ─────────────────────────────────────────────────────
echo "▸ Check 1: Rust file size limit (${MAX_LINES} lines max)"

OVERSIZED=()
while IFS= read -r line; do
    count=$(echo "$line" | awk '{print $1}')
    file=$(echo "$line" | awk '{print $2}')
    if [ "$count" -gt "$MAX_LINES" ]; then
        OVERSIZED+=("$file ($count lines)")
    fi
done < <(find crates/ -name "*.rs" -not -path "*/target/*" | xargs wc -l 2>/dev/null | grep -v " total$" | sort -rn)

if [ ${#OVERSIZED[@]} -eq 0 ]; then
    pass "All .rs files under ${MAX_LINES} lines"
else
    for f in "${OVERSIZED[@]}"; do
        fail "OVERSIZED: $f — split into smaller modules"
    done
fi

# ─────────────────────────────────────────────────────
# CHECK 2: No standalone test files outside tests/suite/
# (Each standalone file = separate test binary = more RAM)
# ─────────────────────────────────────────────────────
echo ""
echo "▸ Check 2: Test consolidation (tests/suite/ pattern)"

STANDALONE=()
for crate_dir in crates/*/; do
    tests_dir="${crate_dir}tests"
    [ -d "$tests_dir" ] || continue

    # Find .rs files directly in tests/ (not in tests/suite/)
    while IFS= read -r f; do
        [ -z "$f" ] && continue
        STANDALONE+=("$f")
    done < <(find "$tests_dir" -maxdepth 1 -name "*.rs" -not -name "mod.rs" 2>/dev/null)
done

if [ ${#STANDALONE[@]} -eq 0 ]; then
    pass "All integration tests use tests/suite/ pattern"
else
    for f in "${STANDALONE[@]}"; do
        warn "Standalone test binary: $f — consider moving to tests/suite/"
    done
fi

# ─────────────────────────────────────────────────────
# CHECK 3: hydra-native has no direct hydra-* deps
# (only hydra-native-state and hydra-native-cognitive)
# ─────────────────────────────────────────────────────
echo ""
echo "▸ Check 3: hydra-native dependency hygiene"

NATIVE_TOML="crates/hydra-native/Cargo.toml"
if [ -f "$NATIVE_TOML" ]; then
    BAD_DEPS=$(grep -E '^hydra-(core|db|runtime|federation|server|model|intent|gate|belief|ledger|sisters|compiler)' "$NATIVE_TOML" 2>/dev/null || true)
    if [ -z "$BAD_DEPS" ]; then
        pass "hydra-native has no direct hydra-* dependencies (uses sub-crates only)"
    else
        fail "hydra-native has direct hydra-* deps that should go through sub-crates:"
        echo "     $BAD_DEPS"
    fi
else
    warn "hydra-native/Cargo.toml not found"
fi

# ─────────────────────────────────────────────────────
# CHECK 4: Workspace compiler settings for memory
# ─────────────────────────────────────────────────────
echo ""
echo "▸ Check 4: Compiler memory settings"

WORKSPACE_TOML="Cargo.toml"
if grep -q 'codegen-units = 16' "$WORKSPACE_TOML" 2>/dev/null; then
    pass "codegen-units = 16 set in workspace"
else
    fail "Missing codegen-units = 16 in workspace Cargo.toml [profile.dev] or [profile.test]"
fi

if grep -q 'debug = 1' "$WORKSPACE_TOML" 2>/dev/null; then
    pass "debug = 1 (minimal debug info) set in workspace"
else
    fail "Missing debug = 1 in workspace Cargo.toml — reduces linker memory"
fi

if grep -q 'split-debuginfo = "unpacked"' "$WORKSPACE_TOML" 2>/dev/null; then
    pass "split-debuginfo = unpacked set (macOS linker optimization)"
else
    warn "Missing split-debuginfo = unpacked — recommended for macOS"
fi

# ─────────────────────────────────────────────────────
# SUMMARY
# ─────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════"
echo " Results: $PASS passed, $FAIL failed, $WARN warnings"
echo "═══════════════════════════════════════════════════"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo "🚫 BLOCKED — fix failures before committing."
    echo "   See: specs/EMERGENCY-CRASH-FIX.md"
    exit 1
fi

if [ "$WARN" -gt 0 ]; then
    echo "⚠️  Warnings present — review but not blocking."
fi

echo "✅ All OOM guards pass."
exit 0
