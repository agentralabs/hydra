#!/usr/bin/env bash
# check-hardening-compliance.sh
# Fails CI if mandatory hardening patterns are absent from source code.
# Part of CANONICAL_SISTER_KIT Section 12 enforcement.

set -euo pipefail

PASS=0
FAIL=0

check() {
    local desc="$1"
    local pattern="$2"
    local path="$3"
    if grep -rq "$pattern" "$path" 2>/dev/null; then
        echo "  ✅ $desc"
        PASS=$((PASS + 1))
    else
        echo "  ❌ MISSING: $desc"
        echo "     Expected pattern: $pattern"
        echo "     Search path:      $path"
        FAIL=$((FAIL + 1))
    fi
}

echo ""
echo "═══════════════════════════════════════════════════"
echo " Hardening Compliance Check — CANONICAL_SISTER_KIT §12"
echo "═══════════════════════════════════════════════════"
echo ""

# Resolve repo root (script may be called from anywhere)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

SRC="$REPO_ROOT/crates"
SCRIPTS="$REPO_ROOT/scripts"
TESTS="$REPO_ROOT/crates"

echo "§12.1  MCP Input Validation"
check "Validation error returned on bad input" \
    "InvalidParams\|invalid_params\|validation_error\|ValidationError" "$SRC"

echo ""
echo "§12.4  Concurrent Startup Lock"
check "PID-based lock file implementation" \
    "lock_path\|LockFile\|InstanceLock\|pid.*lock\|lock.*pid" "$SRC"
check "Stale lock recovery" \
    "stale.*lock\|is_process_alive\|kill.*0\|stale_lock" "$SRC"

echo ""
echo "§12.5  Merge-Only MCP Config"
check "Merge-only config write in installer" \
    "merge\|jq.*argjson\|python3.*json\|update_mcp_config" "$SCRIPTS"

echo ""
echo "§12.9  Server Auth Gate"
check "Token-based auth in server mode" \
    "AGENTIC_TOKEN\|server.*auth\|auth.*token\|require_token" "$SRC"

echo ""
echo "§12.10 Stress Test Suite"
check "Multi-project isolation test" \
    "multi_project\|same_name\|project_isolation\|canonical_path" "$TESTS"
check "Concurrent startup test" \
    "concurrent.*start\|double.*instance\|lock.*concurrent\|already_running" "$TESTS"
check "Restart continuity test" \
    "restart.*continuity\|persist.*restart\|survives.*restart\|state.*restart" "$TESTS"
check "Server auth test" \
    "server.*auth.*test\|missing.*token\|auth.*gate\|unauthorized.*server" "$TESTS"

echo ""
echo "═══════════════════════════════════════════════════"
if [ "$FAIL" -gt 0 ]; then
    echo " RESULT: FAILED ($FAIL missing, $PASS passing)"
    echo ""
    echo " This PR cannot be merged until all hardening checks pass."
    echo " See CANONICAL_SISTER_KIT.md Section 12 for requirements."
    echo "═══════════════════════════════════════════════════"
    exit 1
else
    echo " RESULT: PASSED ($PASS checks, 0 missing)"
    echo "═══════════════════════════════════════════════════"
    exit 0
fi
