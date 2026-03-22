#!/usr/bin/env bash
# check-sister-hardening.sh — Run on every sister PR.
# Usage: ./ci/check-sister-hardening.sh crates/hydra-<sister-name>
# Exit 0 = all checks pass. Exit 1 = one or more failed.

set -euo pipefail

SISTER_PATH="${1:?Usage: $0 <path-to-sister-crate>}"
FAIL=0

pass() { echo "  PASS: $1"; }
fail() { echo "  FAIL: $1"; FAIL=1; }

echo "=== Sister Hardening Check: $SISTER_PATH ==="

# Check 1: MCP parameter validation
if grep -rq "MissingParam\|missing.*param\|required.*param\|param.*required" \
    "$SISTER_PATH/src/" 2>/dev/null; then
    pass "MCP strict parameter validation present"
else
    fail "MCP parameter validation missing (no MissingParam/missing_param pattern)"
fi

# Check 2: Canonical path identity
if grep -rq "canonical\|canonicalize\|hash.*path\|path.*hash" \
    "$SISTER_PATH/src/" 2>/dev/null; then
    pass "Canonical-path identity present"
else
    fail "Canonical-path identity missing"
fi

# Check 3: Startup locking
if grep -rq "\.lock\|LockFile\|flock\|lock_file\|lock_path" \
    "$SISTER_PATH/src/" 2>/dev/null; then
    pass "Startup locking present"
else
    fail "Startup locking missing"
fi

# Check 4: Merge-only MCP config
if grep -rq "merge\|preserve.*existing\|extend.*config\|insert.*if.*not.*exist" \
    "$SISTER_PATH/src/" 2>/dev/null; then
    pass "Merge-only MCP config behavior present"
else
    fail "Merge-only MCP config behavior missing"
fi

# Check 5: Profile-based installer
if grep -rq "desktop\|terminal\|server.*profile\|profile.*server" \
    "$SISTER_PATH/src/" 2>/dev/null; then
    pass "Profile-based installer (desktop|terminal|server) present"
else
    fail "Profile-based installer missing"
fi

# Check 6: Server auth token
if grep -rq "SISTER_AUTH_TOKEN\|auth.*token\|server.*auth\|token.*auth" \
    "$SISTER_PATH/src/" 2>/dev/null; then
    pass "Server auth token gating present"
else
    fail "Server auth token gating missing (SISTER_AUTH_TOKEN or equivalent)"
fi

# Check 7: File size — no file over 400 lines
OVER_400=0
while IFS= read -r -d '' rs_file; do
    line_count=$(wc -l < "$rs_file")
    if [ "$line_count" -gt 400 ]; then
        fail "File over 400 lines: $rs_file ($line_count lines)"
        OVER_400=1
    fi
done < <(find "$SISTER_PATH/src" -name "*.rs" -print0 2>/dev/null)
if [ "$OVER_400" -eq 0 ]; then
    pass "All source files under 400 lines"
fi

echo ""
if [ "$FAIL" -ne 0 ]; then
    echo "RESULT: FAILED — fix the above before merging."
    exit 1
else
    echo "RESULT: ALL CHECKS PASSED"
    exit 0
fi
